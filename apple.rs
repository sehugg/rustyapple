//
// sprocketnes/nes.rs
//
// Author: Patrick Walton
//

#[feature(link_args, macro_rules)];
//#[no_main];

//extern mod native;
//extern mod sdl;

// NB: This must be first to pick up the macro definitions. What a botch.
#[macro_escape]
pub mod util;

#[macro_escape]
pub mod cpu;
pub mod mem;
pub mod a2;
pub mod diskii;

mod tests;

/*
#[no_mangle]
pub extern "C" fn SDL_main(argc: i32, argv: **u8) -> i32 {
    native::start(argc as int, argv, proc() main::start(argc, argv)) as i32
}

*/