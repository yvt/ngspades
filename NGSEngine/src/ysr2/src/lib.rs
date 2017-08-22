//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! YSR2 - *Yonagi* Sound Renderer 2
//! ================================
//!
//! Nomenclature
//! ------------
//!
//! Yonagi (夜凪) refers to the calmness of the sea at dawn. With an absence of
//! loud sounds, all you can hear is the sound of silence. But what causes this
//! perception is not the lack of audible sound, but the constant presence of
//! ambient sounds with an extremely low intensity. In such an environment, the
//! human auditory perception system becomes more sensitive in multiple orders
//! of magnitude and capable of detecting virtually any disturbances at a
//! surprising level of precision of directional cues without any blind spots,
//! which is something that is impossible to achieve with any other perceptions.
//!
//! People tend to underestimate the importance of auditory stimuli. Maybe this
//! misperception can be attributed to the poor performance of auditory
//! recognition memory compared to that of visual memory[^fn1]. Anyway, the
//! point is that, auditory stimuli play an important role in immersive
//! environments.
//!
//! [^fn1]: Michael A. Cohen, Todd S. Horowitz, and Jeremy M. Wolfe,
//!     "[Auditory recognition memory is inferior to visual recognition memory]
//!     (http://www.pnas.org/content/106/14/6008.full), "
//!     PNAS 2009 106 (14) 6008-6010; published ahead of print March 23, 2009,
//!     doi:10.1073/pnas.0811884106
//!
//! <!--
//! Nah, I just wanted to make a reference to The Sound of Silence. It is an old
//! song by Simon and Garfunkel. It is in no way related to [some fan art] based
//! on a certain cartoon.
//!
//! [some fan art]: https://derpibooru.org/1266994
//! -->
//!
//! Building
//! --------
//!
//! Examples of YSR2 have a dependency on the `portaudio` crate. It is merely a
//! binding to the PortAudio library, so you have to install the library in one
//! of the following ways:
//!
//!  - **On Linux:** No actions are necessary; `portaudio` comes with a
//!    `build.rs` which downloads and builds PortAudio automatically.
//!  - **On Windows:** You have to [download PortAudio] and build it by yourself.
//!    After that, you must copy the built `portaudio.lib` to the `target/*/deps`
//!    directory.
//!  - **On macOS:** You have to install `portaudio` and `pkg-config`. Using
//!    [Homebrew], this can be done by running `brew install portaudio pkg-config`.
//!
//! Ideally, we should not have to do this manually. For example, we could put a
//! copy of the PortAudio source tree into ours and integrate it into the Cargo
//! build system by using the `gcc` crate.
//!
//! [download PortAudio]: http://www.portaudio.com/download.html
//! [Homebrew]: https://brew.sh
pub extern crate ysr2_common as common;
pub extern crate ysr2_localizer as localizer;
pub extern crate ysr2_mixer as mixer;
