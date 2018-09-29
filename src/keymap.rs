pub const KEYMAP_DATA: [[char; 3]; 4] =
    [ ['1', '2', '3']
    , ['4', '5', '6']
    , ['7', '8', '9']
    , ['a', '0', 'b']
    ];


pub const KEYMAP: [&'static [char]; 4] =
    [ &KEYMAP_DATA[0]
    , &KEYMAP_DATA[1]
    , &KEYMAP_DATA[2]
    , &KEYMAP_DATA[3]
    ];
