// The following utils have been adapted from various projects. It is not recommended to directly
// copy and paste them to your own project without a detailed understanding of what they do
// and why it is necessary for HDR to implement them.

#[macro_export]
macro_rules! dump_trace {
    () => {{
        let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
        println!("Current text: {:#x}", text);

        let mut lr: *const u64;
        unsafe {
            asm!("mov $0, x30" : "=r"(lr) : : : "volatile");
        }
        let mut fp: *const u64;
        unsafe {
            asm!("mov $0, x29" : "=r"(fp) : : : "volatile");
        }

        println!("Current LR: {:#X}", (lr as u64));

        while !fp.is_null() {
            lr = *fp.offset(1) as *const u64;
            if !lr.is_null() {
                println!("LR: {:#x}", (lr as u64) - text);
            }
            fp = *fp as *const u64;
        }
    }}
}

#[macro_export]
macro_rules! debugln {
    ($($args:expr),*) => {{
        if cfg!(feature = "debug") {
            println!($($args),*);
        }
    }}
}

#[macro_export]
macro_rules! c_str {
    ($l:tt) => {
        [$l.as_bytes(), "\u{0}".as_bytes()].concat().as_ptr();
    };
}

#[macro_export]
macro_rules! vtable_addr {
    ($obj:ident) => {
        (*($obj as *const u64) - (skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64))
    }
}

pub unsafe fn byte_search(needle: &[u32]) -> Option<usize> {
    let mut matching = 0usize;
    let text_start = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32;
    let text_end = skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as *const u32;
    let mut pos = 0isize;
    let mut match_begin = 0usize;
    loop {
        if text_start.offset(pos) == text_end { break; }
        if matching == needle.len() { break; }
        if *text_start.offset(pos) == needle[matching] {
            if matching == 0 { match_begin = text_start.offset(pos) as usize; }
            matching += 1;
        }
        else {
            matching = 0;
            match_begin = 0;
        }
        pos += 1;
    }
    if match_begin == 0 { None }
    else { Some(match_begin) }
}

// similar to above but for u8 values
pub fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn offset_from_adrp(adrp_offset: usize) -> usize {
    unsafe {
        let adrp = *(offset_to_addr(adrp_offset) as *const u32);
        let immhi = (adrp & 0b0_00_00000_1111111111111111111_00000) >> 3;
        let immlo = (adrp & 0b0_11_00000_0000000000000000000_00000) >> 29;
        let imm = ((immhi | immlo) << 12) as i32 as usize;
        let base = adrp_offset & 0xFFFFFFFFFFFFF000;
        base + imm
    }
}

pub fn offset_from_ldr(ldr_offset: usize) -> usize {
    unsafe {
        let ldr = *(offset_to_addr(ldr_offset) as *const u32);
        let size = (ldr & 0b11_000_0_00_00_000000000000_00000_00000) >> 30;
        let imm = (ldr & 0b00_000_0_00_00_111111111111_00000_00000) >> 10;
        (imm as usize) << size
    }
}

pub fn offset_to_addr(offset: usize) -> *const () {
    unsafe { (skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u8).add(offset) as _ }
}

pub fn agent_to_agent_kind<S: Into<String>>(agent: S) -> i32 {
    let mut agent: String = agent.into();
    agent.make_ascii_uppercase();
    let mut kind = -1; // FIGHTER_KIND_NONE
    unsafe {
        if !smash::lib::lua_bind_get_value(lua_bind_hash::lua_bind_hash_str(format!("FIGHTER_KIND_{}", agent)), &mut kind)
            && !smash::lib::lua_bind_get_value(lua_bind_hash::lua_bind_hash_str(format!("WEAPON_KIND_{}", agent)), &mut kind)
            && !smash::lib::lua_bind_get_value(lua_bind_hash::lua_bind_hash_str(format!("ITEM_KIND_{}", agent)), &mut kind) // hail mary
            {}
    }
    kind
}