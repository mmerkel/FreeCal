Only for human use, don't touch.

- [x] check the mechanism of creating the typescript types. Is this the right one?
- [ ] time zone format in which events are stored - let's discuss!
- [ ] default time format to 24h format
- UI:
  - [ ] events should indicate event color when not all day:  i.e. by the font color
  - [ ] event display dialog: add buttons as icons on top right of dialog, like Google calendar - dialog or a new thing?  Or add at least a cross on top right, which always means cancel
  - [ ] event edit dialog:
    - [ ] when clicking on date, little calendar appears, this is great, but also some gray rectangle, which should disappear.
    - [ ] setting the time is too cumbersome
      - [ ] simplify with some widget
      - [ ] time: allow for more flexible entry: 
        - [ ] if entered in 24h format, convert to AM/PM (if this is given), and vice versa
        - [ ] if set to AM/PM: pressing A/P at any time changes format
  - [ ] links also for location, similar to description
  - [ ] cursor: when hovering over event text should not become edit cursor, maybe hand like in Google calendar
  - [ ] resizing of window:
    - [ ] currently, there is no reasonable minimum size, what can be done?
    - [ ] there is a lag, can this be improved?
