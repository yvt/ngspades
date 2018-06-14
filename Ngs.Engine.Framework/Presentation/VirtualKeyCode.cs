//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Represents a symbolic name for a specific key.
    /// </summary>
    /// <remarks>
    /// This enumerate type is based on <see href="https://github.com/tomaka/winit">winit</see>'s
    /// <c>VirtualKeyCode</c>.
    /// </remarks>
    public enum VirtualKeyCode {
        /// <summary>
        /// The '1' key.
        /// </summary>
        Key1,
        /// <summary>
        /// The '2' key.
        /// </summary>
        Key2,
        /// <summary>
        /// The '3' key.
        /// </summary>
        Key3,
        /// <summary>
        /// The '4' key.
        /// </summary>
        Key4,
        /// <summary>
        /// The '5' key.
        /// </summary>
        Key5,
        /// <summary>
        /// The '6' key.
        /// </summary>
        Key6,
        /// <summary>
        /// The '7' key.
        /// </summary>
        Key7,
        /// <summary>
        /// The '8' key.
        /// </summary>
        Key8,
        /// <summary>
        /// The '9' key.
        /// </summary>
        Key9,
        /// <summary>
        /// The '0' key.
        /// </summary>
        Key0,

        /// <summary>
        /// The 'A' key.
        /// </summary>
        A,
        /// <summary>
        /// The 'B' key.
        /// </summary>
        B,
        /// <summary>
        /// The 'C' key.
        /// </summary>
        C,
        /// <summary>
        /// The 'D' key.
        /// </summary>
        D,
        /// <summary>
        /// The 'E' key.
        /// </summary>
        E,
        /// <summary>
        /// The 'F' key.
        /// </summary>
        F,
        /// <summary>
        /// The 'G' key.
        /// </summary>
        G,
        /// <summary>
        /// The 'H' key.
        /// </summary>
        H,
        /// <summary>
        /// The 'I' key.
        /// </summary>
        I,
        /// <summary>
        /// The 'J' key.
        /// </summary>
        J,
        /// <summary>
        /// The 'K' key.
        /// </summary>
        K,
        /// <summary>
        /// The 'L' key.
        /// </summary>
        L,
        /// <summary>
        /// The 'M' key.
        /// </summary>
        M,
        /// <summary>
        /// The 'N' key.
        /// </summary>
        N,
        /// <summary>
        /// The 'O' key.
        /// </summary>
        O,
        /// <summary>
        /// The 'P' key.
        /// </summary>
        P,
        /// <summary>
        /// The 'Q' key.
        /// </summary>
        Q,
        /// <summary>
        /// The 'R' key.
        /// </summary>
        R,
        /// <summary>
        /// The 'S' key.
        /// </summary>
        S,
        /// <summary>
        /// The 'T' key.
        /// </summary>
        T,
        /// <summary>
        /// The 'U' key.
        /// </summary>
        U,
        /// <summary>
        /// The 'V' key.
        /// </summary>
        V,
        /// <summary>
        /// The 'W' key.
        /// </summary>
        W,
        /// <summary>
        /// The 'X' key.
        /// </summary>
        X,
        /// <summary>
        /// The 'Y' key.
        /// </summary>
        Y,
        /// <summary>
        /// The 'Z' key.
        /// </summary>
        Z,

        /// <summary>
        /// The 'escape' key.
        /// </summary>
        Escape,

        /// <summary>
        /// The 'F1' key.
        /// </summary>
        F1,
        /// <summary>
        /// The 'F2' key.
        /// </summary>
        F2,
        /// <summary>
        /// The 'F3' key.
        /// </summary>
        F3,
        /// <summary>
        /// The 'F4' key.
        /// </summary>
        F4,
        /// <summary>
        /// The 'F5' key.
        /// </summary>
        F5,
        /// <summary>
        /// The 'F6' key.
        /// </summary>
        F6,
        /// <summary>
        /// The 'F7' key.
        /// </summary>
        F7,
        /// <summary>
        /// The 'F8' key.
        /// </summary>
        F8,
        /// <summary>
        /// The 'F9' key.
        /// </summary>
        F9,
        /// <summary>
        /// The 'F10' key.
        /// </summary>
        F10,
        /// <summary>
        /// The 'F11' key.
        /// </summary>
        F11,
        /// <summary>
        /// The 'F12' key.
        /// </summary>
        F12,
        /// <summary>
        /// The 'F13' key.
        /// </summary>
        F13,
        /// <summary>
        /// The 'F14' key.
        /// </summary>
        F14,
        /// <summary>
        /// The 'F15' key.
        /// </summary>
        F15,

        /// <summary>
        /// The print screen/system request key.
        /// </summary>
        Snapshot,
        /// <summary>
        /// The scroll lock key.
        /// </summary>
        Scroll,
        /// <summary>
        /// The pause/break key.
        /// </summary>
        Pause,
        /// <summary>
        /// The insert key.
        /// </summary>
        Insert,
        /// <summary>
        /// The home key.
        /// </summary>
        Home,
        /// <summary>
        /// The delete key.
        /// </summary>
        Delete,
        /// <summary>
        /// The end key.
        /// </summary>
        End,
        /// <summary>
        /// The page down key.
        /// </summary>
        PageDown,
        /// <summary>
        /// The page up key.
        /// </summary>
        PageUp,

        /// <summary>
        /// The left arrow key.
        /// </summary>
        Left,
        /// <summary>
        /// The up arrow key.
        /// </summary>
        Up,
        /// <summary>
        /// The right arrow key.
        /// </summary>
        Right,
        /// <summary>
        /// The down arrow key.
        /// </summary>
        Down,

        /// <summary>
        /// The back key.
        /// </summary>
        Back,
        /// <summary>
        /// The return key.
        /// </summary>
        Return,
        /// <summary>
        /// The space key.
        /// </summary>
        Space,
        /// <summary>
        /// The compose key.
        /// </summary>
        Compose,
        /// <summary>
        /// The caret key.
        /// </summary>
        Caret,
        /// <summary>
        /// The num lock key.
        /// </summary>
        Numlock,

        /// <summary>
        /// The '0' key on the numeric keypad.
        /// </summary>
        Numpad0,
        /// <summary>
        /// The '1' key on the numeric keypad.
        /// </summary>
        Numpad1,
        /// <summary>
        /// The '2' key on the numeric keypad.
        /// </summary>
        Numpad2,
        /// <summary>
        /// The '3' key on the numeric keypad.
        /// </summary>
        Numpad3,
        /// <summary>
        /// The '4' key on the numeric keypad.
        /// </summary>
        Numpad4,
        /// <summary>
        /// The '5' key on the numeric keypad.
        /// </summary>
        Numpad5,
        /// <summary>
        /// The '6' key on the numeric keypad.
        /// </summary>
        Numpad6,
        /// <summary>
        /// The '7' key on the numeric keypad.
        /// </summary>
        Numpad7,
        /// <summary>
        /// The '8' key on the numeric keypad.
        /// </summary>
        Numpad8,
        /// <summary>
        /// The '9' key on the numeric keypad.
        /// </summary>
        Numpad9,

        /// <summary>
        /// The 'ABNT_C1' (Brazilian) key.
        /// </summary>
        AbntC1,
        /// <summary>
        /// The 'ABNT_C2' (Brazilian) key.
        /// </summary>
        AbntC2,
        /// <summary>
        /// The add key.
        /// </summary>
        Add,
        /// <summary>
        /// The apostrophe key.
        /// </summary>
        Apostrophe,
        /// <summary>
        /// The application key (Microsoft Natural Keyboard).
        /// </summary>
        Apps,
        /// <summary>
        /// The at key.
        /// </summary>
        At,
        /// <summary>
        /// The ax key.
        /// </summary>
        Ax,
        /// <summary>
        /// The backslash key.
        /// </summary>
        Backslash,
        /// <summary>
        /// The calculator key.
        /// </summary>
        Calculator,
        /// <summary>
        /// The capital key.
        /// </summary>
        Capital,
        /// <summary>
        /// The colon key.
        /// </summary>
        Colon,
        /// <summary>
        /// The comma key.
        /// </summary>
        Comma,
        /// <summary>
        /// The convert key.
        /// </summary>
        Convert,
        /// <summary>
        /// The decimal key.
        /// </summary>
        Decimal,
        /// <summary>
        /// The divide key.
        /// </summary>
        Divide,
        /// <summary>
        /// The equals key.
        /// </summary>
        Equals,
        /// <summary>
        /// The grave key.
        /// </summary>
        Grave,
        /// <summary>
        /// The kana key.
        /// </summary>
        Kana,
        /// <summary>
        /// The kanji key.
        /// </summary>
        Kanji,

        /// <summary>
        /// The left alt key.
        /// </summary>
        LAlt,
        /// <summary>
        /// The left bracket key.
        /// </summary>
        LBracket,
        /// <summary>
        /// The left control key.
        /// </summary>
        LControl,
        /// <summary>
        /// The left shift key.
        /// </summary>
        LShift,
        /// <summary>
        /// The left meta key (e.g., the Windows logo key in Windows, Command in macOS).
        /// </summary>
        LWin,

        /// <summary>
        /// The mail key.
        /// </summary>
        Mail,
        /// <summary>
        /// The media select key.
        /// </summary>
        MediaSelect,
        /// <summary>
        /// The media stop key.
        /// </summary>
        MediaStop,
        /// <summary>
        /// The minus key.
        /// </summary>
        Minus,
        /// <summary>
        /// The multiply key.
        /// </summary>
        Multiply,
        /// <summary>
        /// The mute key.
        /// </summary>
        Mute,
        /// <summary>
        /// The "my computer" key.
        /// </summary>
        MyComputer,
        /// <summary>
        /// The navigate forward key.
        /// </summary>
        NavigateForward,
        /// <summary>
        /// The navigate backward key.
        /// </summary>
        NavigateBackward,
        /// <summary>
        /// The next track key.
        /// </summary>
        NextTrack,
        /// <summary>
        /// The "no convert" key.
        /// </summary>
        NoConvert,

        /// <summary>
        /// The comma key on the numeric keypad.
        /// </summary>
        NumpadComma,
        /// <summary>
        /// The enter key on the numeric keypad.
        /// </summary>
        NumpadEnter,
        /// <summary>
        /// The equals key on the numeric keypad.
        /// </summary>
        NumpadEquals,

        /// <summary>
        /// The OEM 102 key.
        /// </summary>
        OEM102,
        /// <summary>
        /// The period key.
        /// </summary>
        Period,
        /// <summary>
        /// The play/pause key.
        /// </summary>
        PlayPause,
        /// <summary>
        /// The power key.
        /// </summary>
        Power,
        /// <summary>
        /// The prev track key.
        /// </summary>
        PrevTrack,

        /// <summary>
        /// The right alt key.
        /// </summary>
        RAlt,
        /// <summary>
        /// The right bracket key.
        /// </summary>
        RBracket,
        /// <summary>
        /// The right control key.
        /// </summary>
        RControl,
        /// <summary>
        /// The right shift key.
        /// </summary>
        RShift,
        /// <summary>
        /// The right meta key (e.g., the Windows logo key in Windows, Command in macOS).
        /// </summary>
        RWin,

        /// <summary>
        /// The semicolon key.
        /// </summary>
        Semicolon,
        /// <summary>
        /// The slash key.
        /// </summary>
        Slash,
        /// <summary>
        /// The sleep key.
        /// </summary>
        Sleep,
        /// <summary>
        /// The stop key.
        /// </summary>
        Stop,
        /// <summary>
        /// The subtract key.
        /// </summary>
        Subtract,
        /// <summary>
        /// The system request key.
        /// </summary>
        Sysrq,
        /// <summary>
        /// The tab key.
        /// </summary>
        Tab,
        /// <summary>
        /// The underline key.
        /// </summary>
        Underline,
        /// <summary>
        /// The unlabeled key.
        /// </summary>
        Unlabeled,
        /// <summary>
        /// The volume down key.
        /// </summary>
        VolumeDown,
        /// <summary>
        /// The volume up key.
        /// </summary>
        VolumeUp,
        /// <summary>
        /// The wake key.
        /// </summary>
        Wake,
        /// <summary>
        /// The Browser back key.
        /// </summary>
        WebBack,
        /// <summary>
        /// The Browser favorites key.
        /// </summary>
        WebFavorites,
        /// <summary>
        /// The Browser forward key.
        /// </summary>
        WebForward,
        /// <summary>
        /// The Browser home key.
        /// </summary>
        WebHome,
        /// <summary>
        /// The Browser refresh key.
        /// </summary>
        WebRefresh,
        /// <summary>
        /// The Browser search key.
        /// </summary>
        WebSearch,
        /// <summary>
        /// The Browser stop key.
        /// </summary>
        WebStop,
        /// <summary>
        /// The yen key.
        /// </summary>
        Yen,
        /// <summary>
        /// The copy key. (X11 <c>XF86XK_Copy</c>)
        /// </summary>
        Copy,
        /// <summary>
        /// The paste key. (X11 <c>XF86XK_Paste</c>)
        /// </summary>
        Paste,
        /// <summary>
        /// The cut key. (X11 <c>XF86XK_Cut</c>)
        /// </summary>
        Cut,
    }
}