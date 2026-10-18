//! The CalDAV client: WebDAV requests (RFC 4918) to a CalDAV server
//! (RFC 4791). Request bodies are written with an XML writer and answers
//! read with a namespace-aware XML reader, never built or searched as
//! strings. Everything a server sends is untrusted.

use std::time::Duration;

use base64::Engine;
use quick_xml::events::Event as XmlEvent;
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::{NsReader, Writer};
use url::Url;

use crate::{CalDavLogin, CoreError, Result};

const DAV: &str = "DAV:";

/// The longest server text passed on in an error, in characters.
const MAX_MESSAGE: usize = 200;

/// A WebDAV property: its namespace and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Property {
    namespace: &'static str,
    name: &'static str,
}

/// The URL of the principal of the user who is logged in (RFC 5397).
const CURRENT_USER_PRINCIPAL: Property = Property {
    namespace: DAV,
    name: "current-user-principal",
};

/// A connection to one CalDAV server, logged in as one user.
pub(crate) struct Client {
    agent: ureq::Agent,
    url: Url,
    authorization: String,
}

impl Client {
    pub fn new(login: &CalDavLogin) -> Result<Self> {
        let url = Url::parse(login.url.trim()).map_err(|_| CoreError::InvalidServerUrl)?;
        if !matches!(url.scheme(), "http" | "https") || !url.has_host() {
            return Err(CoreError::InvalidServerUrl);
        }
        // Basic authentication splits at the first colon, so a username
        // with one would log in as someone else, or no one.
        if login.username.contains(':') {
            return Err(CoreError::WrongCredentials);
        }
        let credentials = format!("{}:{}", login.username, login.password);
        let agent = ureq::Agent::config_builder()
            // Answers with an error status are read, not failed.
            .http_status_as_error(false)
            // WebDAV's methods, such as PROPFIND, aren't HTTP's own.
            .allow_non_standard_methods(true)
            // Redirects aren't followed, so that the password only ever goes
            // to the server the user named.
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(60)))
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::NativeTls)
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .new_agent();
        Ok(Self {
            agent,
            url,
            authorization: format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(credentials)
            ),
        })
    }

    /// The server's URL, as given.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The URL of the principal of the user who is logged in. Asking for it
    /// checks the username and password, and that the server speaks WebDAV.
    pub fn current_user_principal(&self) -> Result<Url> {
        let multistatus = self.propfind(&self.url, &[CURRENT_USER_PRINCIPAL])?;
        multistatus
            .property(CURRENT_USER_PRINCIPAL)
            .and_then(|principal| principal.child(DAV, "href"))
            .and_then(|href| self.url.join(href.text.trim()).ok())
            .ok_or(CoreError::NotCalDavServer)
    }

    /// Asks for `properties` of the resource at `url` alone (`Depth: 0`).
    fn propfind(&self, url: &Url, properties: &[Property]) -> Result<Element> {
        let request = ureq::http::Request::builder()
            .method(ureq::http::Method::from_bytes(b"PROPFIND").unwrap())
            .uri(url.as_str())
            .header("Authorization", &self.authorization)
            .header("Depth", "0")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(propfind_body(properties))
            .map_err(|_| CoreError::InvalidServerUrl)?;
        let response = self.agent.run(request).map_err(transport_error)?;
        let status = response.status().as_u16();
        let plain_text = response
            .headers()
            .get("Content-Type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.trim_start().starts_with("text/plain"));
        let body = response.into_body().read_to_string().unwrap_or_default();
        match status {
            207 => parse(&body)
                .filter(|root| root.is(DAV, "multistatus"))
                .ok_or(CoreError::NotCalDavServer),
            401 => Err(CoreError::WrongCredentials),
            200..=299 => Err(CoreError::NotCalDavServer),
            _ => Err(CoreError::ServerRefused {
                status,
                message: if plain_text {
                    plain_message(&body)
                } else {
                    String::new()
                },
            }),
        }
    }
}

/// A failure before the server answered: being offline, or a secure
/// connection that couldn't be made.
fn transport_error(error: ureq::Error) -> CoreError {
    match error {
        ureq::Error::Tls(_) | ureq::Error::NativeTls(_) | ureq::Error::Der(_) => {
            CoreError::SecureConnectionFailed(error.to_string())
        }
        ureq::Error::Protocol(_) | ureq::Error::Http(_) => CoreError::NotCalDavServer,
        error => CoreError::ServerUnreachable(error.to_string()),
    }
}

/// A server's own text, made safe to pass on: control and bidi formatting
/// characters become spaces, whitespace runs one space, and it is cut to
/// [`MAX_MESSAGE`] characters. It stays untrusted text.
fn plain_message(text: &str) -> String {
    let spaced: String = text
        .chars()
        .map(|c| match c {
            '\u{200e}'
            | '\u{200f}'
            | '\u{061c}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_MESSAGE)
        .collect()
}

/// A `PROPFIND` request body asking for `properties`.
fn propfind_body(properties: &[Property]) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer
        .create_element("propfind")
        .with_attribute(("xmlns", DAV))
        .write_inner_content(|writer| {
            writer
                .create_element("prop")
                .write_inner_content(|writer| {
                    for (index, property) in properties.iter().enumerate() {
                        // Each property declares its own namespace on a
                        // prefix of its own.
                        let prefix = format!("p{index}");
                        writer
                            .create_element(format!("{prefix}:{}", property.name))
                            .with_attribute((
                                format!("xmlns:{prefix}").as_str(),
                                property.namespace,
                            ))
                            .write_empty()?;
                    }
                    Ok(())
                })?;
            Ok(())
        })
        .expect("writing to memory doesn't fail");
    let mut body = br#"<?xml version="1.0" encoding="utf-8"?>"#.to_vec();
    body.extend(writer.into_inner());
    body
}

/// An XML element as read from an answer, with its namespace resolved.
#[derive(Debug, Default, PartialEq)]
struct Element {
    namespace: String,
    name: String,
    /// Its text, without that of its children.
    text: String,
    children: Vec<Element>,
}

impl Element {
    fn is(&self, namespace: &str, name: &str) -> bool {
        self.namespace == namespace && self.name == name
    }

    fn child(&self, namespace: &str, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.is(namespace, name))
    }

    fn children<'a>(
        &'a self,
        namespace: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a Element> {
        self.children
            .iter()
            .filter(move |child| child.is(namespace, name))
    }

    /// In a `multistatus`, the first value of `property` that a response
    /// found (status 200).
    fn property(&self, property: Property) -> Option<&Element> {
        self.children(DAV, "response")
            .flat_map(|response| response.children(DAV, "propstat"))
            .filter(|propstat| {
                propstat
                    .child(DAV, "status")
                    .is_some_and(|status| status.text.split_whitespace().nth(1) == Some("200"))
            })
            .filter_map(|propstat| propstat.child(DAV, "prop"))
            .find_map(|prop| prop.child(property.namespace, property.name))
    }
}

/// Reads an answer into its root element, or nothing if it isn't
/// well-formed XML.
fn parse(xml: &str) -> Option<Element> {
    let mut reader = NsReader::from_str(xml);
    let mut open: Vec<Element> = Vec::new();
    let element = |namespace: ResolveResult, local: &[u8]| Element {
        namespace: match namespace {
            ResolveResult::Bound(Namespace(namespace)) => {
                String::from_utf8_lossy(namespace).into_owned()
            }
            _ => String::new(),
        },
        name: String::from_utf8_lossy(local).into_owned(),
        ..Element::default()
    };
    loop {
        let (namespace, event) = reader.read_resolved_event().ok()?;
        match event {
            XmlEvent::Start(start) => open.push(element(namespace, start.local_name().as_ref())),
            XmlEvent::Empty(empty) => {
                let child = element(namespace, empty.local_name().as_ref());
                match open.last_mut() {
                    Some(parent) => parent.children.push(child),
                    None => return Some(child),
                }
            }
            XmlEvent::End(_) => {
                let closed = open.pop()?;
                match open.last_mut() {
                    Some(parent) => parent.children.push(closed),
                    None => return Some(closed),
                }
            }
            // Text outside the root element, such as the line break after
            // the XML declaration, is left out.
            XmlEvent::Text(text) => {
                if let Some(parent) = open.last_mut() {
                    parent.text.push_str(&text.decode().ok()?);
                }
            }
            XmlEvent::CData(data) => {
                if let Some(parent) = open.last_mut() {
                    parent.text.push_str(&data.decode().ok()?);
                }
            }
            // Only character references and XML's five entities: an answer
            // can't declare entities of its own.
            XmlEvent::GeneralRef(reference) => {
                let parent = open.last_mut()?;
                match reference.resolve_char_ref().ok()? {
                    Some(character) => parent.text.push(character),
                    None => parent.text.push_str(quick_xml::escape::resolve_xml_entity(
                        &reference.decode().ok()?,
                    )?),
                }
            }
            XmlEvent::Eof => return None,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_propfind_body_names_each_property_in_its_namespace() {
        let body = String::from_utf8(propfind_body(&[CURRENT_USER_PRINCIPAL])).unwrap();

        let root = parse(&body).unwrap();

        assert!(root.is(DAV, "propfind"));
        let prop = root.child(DAV, "prop").unwrap();
        assert_eq!(prop.children.len(), 1);
        assert!(prop.children[0].is(DAV, "current-user-principal"));
    }

    #[test]
    fn answers_are_read_by_namespace_whatever_their_prefixes() {
        for answer in [
            r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/</d:href><d:propstat>
               <d:prop><d:current-user-principal><d:href>/alice/</d:href></d:current-user-principal></d:prop>
               <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#,
            r#"<multistatus xmlns="DAV:"><response><href>/</href><propstat>
               <prop><current-user-principal><href>/alice/</href></current-user-principal></prop>
               <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
        ] {
            let root = parse(answer).unwrap();

            let principal = root.property(CURRENT_USER_PRINCIPAL).unwrap();

            assert_eq!(principal.child(DAV, "href").unwrap().text, "/alice/");
        }
    }

    #[test]
    fn an_answer_may_start_with_an_xml_declaration() {
        let root = parse("<?xml version='1.0' encoding='utf-8'?>\n<multistatus xmlns=\"DAV:\"/>\n")
            .unwrap();

        assert!(root.is(DAV, "multistatus"));
    }

    #[test]
    fn a_property_the_server_didnt_find_isnt_read() {
        let root = parse(
            r#"<multistatus xmlns="DAV:"><response><href>/</href><propstat>
               <prop><current-user-principal><href>/alice/</href></current-user-principal></prop>
               <status>HTTP/1.1 404 Not Found</status></propstat></response></multistatus>"#,
        )
        .unwrap();

        assert_eq!(root.property(CURRENT_USER_PRINCIPAL), None);
    }

    #[test]
    fn the_same_name_in_another_namespace_isnt_the_property() {
        let root = parse(
            r#"<multistatus xmlns="DAV:" xmlns:x="urn:other"><response><href>/</href><propstat>
               <prop><x:current-user-principal><href>/alice/</href></x:current-user-principal></prop>
               <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
        )
        .unwrap();

        assert_eq!(root.property(CURRENT_USER_PRINCIPAL), None);
    }

    #[test]
    fn text_is_unescaped() {
        let root = parse("<a xmlns='DAV:'>x &amp; y &#x3c;z&gt;</a>").unwrap();

        assert_eq!(root.text, "x & y <z>");
    }

    #[test]
    fn malformed_xml_is_nothing() {
        assert_eq!(parse("<a><b></a>"), None);
        assert_eq!(parse("<html>Welcome"), None);
    }
}
