Only for human use, don't touch.

- [x] check the mechanism of creating the typescript types. Is this the right one?
- [x] time zone format in which events are stored - let's discuss!
- [x] default time format to 24h format
- UI:
  - [x] events should indicate event color when not all day:  i.e. by the font color
  - [x] event display dialog: add buttons as icons on top right of dialog, like Google calendar - dialog or a new thing?  Or add at least a cross on top right, which always means cancel
  - [x] event edit dialog:
    - [x] when clicking on date, little calendar appears, this is great, but also some gray rectangle, which should disappear.
    - [x] setting the time is too cumbersome
      - [x] simplify with some widget
      - [x] time: allow for more flexible entry: 
        - [x] if entered in 24h format, convert to AM/PM (if this is given), and vice versa
        - [x] if set to AM/PM: pressing A/P at any time changes format
  - [x] links also for location, similar to description
  - [x] cursor: when hovering over event text should not become edit cursor, maybe hand like in Google calendar
  - [x] resizing of window:
    - [x] currently, there is no reasonable minimum size, what can be done?
    - [x] there is a lag, can this be improved?
