#![feature(proc_macro_hygiene)]
#![feature(slice_fill)]
#![feature(asm)]
#![feature(new_uninit)]
#![feature(vec_into_raw_parts)]
pub mod fs;
pub mod modules;
pub mod singletons;
pub mod utils;
pub mod vars;

fn callback(path: String, data: Vec<u8>) {}

pub fn nro_hook(info: &skyline::nro::NroInfo) {
    fs::load_associated_files(info);
}

pub fn nro_unhook(info: &skyline::nro::NroInfo) {
    modules::ParamModule::handle_param_unload(info);
    // modules::anim::handle_nuanmb_unload(info);
}

pub fn init() {
    // arc_runtime::init();
    modules::init();
    singletons::init();
    fs::init();
    skyline::nro::add_hook(nro_hook);
    skyline::nro::add_unload_hook(nro_unhook);
}