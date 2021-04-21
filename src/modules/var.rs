use smash::app::BattleObjectModuleAccessor;
use super::VAR_MODULE_OFFSET;

macro_rules! get_var_module {
    ($boma:ident) => {{
        let vtable = *($boma as *const *const u64);
        &mut *(*vtable.offset((*vtable.offset(-1) as isize) + VAR_MODULE_OFFSET) as *mut VarModule)
    }}
}


pub struct VarModule {
    // allocated dynamically anyways, it's fine
    common_int:   [i32; 0x1000],
    common_int64: [u64; 0x1000],
    common_float: [f32; 0x1000],
    common_flag:  [bool; 0x1000],

    fighter_int:   [i32; 0x1000],
    fighter_int64: [u64; 0x1000],
    fighter_float: [f32; 0x1000],
    fighter_flag:  [bool; 0x1000]
}

impl VarModule {
    pub const RESET_COMMON_INT:    u8 = 0b00000001;
    pub const RESET_COMMON_INT64:  u8 = 0b00000010;
    pub const RESET_COMMON_FLOAT:  u8 = 0b00000100;
    pub const RESET_COMMON_FLAG:   u8 = 0b00001000;

    pub const RESET_FIGHTER_INT:   u8 = 0b00010000;
    pub const RESET_FIGHTER_INT64: u8 = 0b00100000;
    pub const RESET_FIGHTER_FLOAT: u8 = 0b01000000;
    pub const RESET_FIGHTER_FLAG:  u8 = 0b10000000;

    pub const RESET_COMMON:   u8 = 0xF;
    pub const RESET_FIGHTER: u8 = 0xF0;
    pub const RESET_ALL:     u8 = 0xFF;
    pub fn new() -> Self {
        Self {
            common_int: [0; 0x1000],
            common_int64: [0; 0x1000],
            common_float: [0.0; 0x1000],
            common_flag: [false; 0x1000],

            fighter_int: [0; 0x1000],
            fighter_int64: [0; 0x1000],
            fighter_float: [0.0; 0x1000],
            fighter_flag: [false; 0x1000]
        }
    }

    fn _reset(&mut self, reset_mask: u8) {
        if reset_mask & Self::RESET_COMMON_INT != 0 {
            self.common_int.fill(0);    
        }
        if reset_mask & Self::RESET_COMMON_INT64 != 0 {
            self.common_int64.fill(0);    
        }
        if reset_mask & Self::RESET_COMMON_FLOAT != 0 {
            self.common_float.fill(0.0);    
        }
        if reset_mask & Self::RESET_COMMON_FLAG != 0 {
            self.common_flag.fill(false);    
        }
        if reset_mask & Self::RESET_FIGHTER_INT != 0 {
            self.fighter_int.fill(0);    
        }
        if reset_mask & Self::RESET_FIGHTER_INT64 != 0 {
            self.fighter_int64.fill(0);    
        }
        if reset_mask & Self::RESET_FIGHTER_FLOAT != 0 {
            self.fighter_float.fill(0.0);    
        }
        if reset_mask & Self::RESET_FIGHTER_FLAG != 0 {
            self.fighter_flag.fill(false);    
        }
    }

    fn _get_int(&mut self, what: i32) -> i32 {
        if what & 0x1000 != 0 {
            self.fighter_int[(what & 0xFFF) as usize]
        }
        else {
            self.common_int[(what & 0xFFF) as usize]
        }
    }
    fn _get_int64(&mut self, what: i32) -> u64 {
        if what & 0x1000 != 0 {
            self.fighter_int64[(what & 0xFFF) as usize]
        }
        else {
            self.common_int64[(what & 0xFFF) as usize]
        }

    }
    fn _get_float(&mut self, what: i32) -> f32 {
        if what & 0x1000 != 0 {
            self.fighter_float[(what & 0xFFF) as usize]
        }
        else {
            self.common_float[(what & 0xFFF) as usize]
        }

    }
    fn _is_flag(&mut self, what: i32) -> bool {
        if what & 0x1000 != 0 {
            self.fighter_flag[(what & 0xFFF) as usize]
        }
        else {
            self.common_flag[(what & 0xFFF) as usize]
        }

    }

    fn _set_int(&mut self, what: i32, val: i32) {
        if what & 0x1000 != 0 {
            self.fighter_int[(what & 0xFFF) as usize] = val;
        }
        else {
            self.common_int[(what & 0xFFF) as usize] = val;
        }
    }
    fn _set_int64(&mut self, what: i32, val: u64) {
        if what & 0x1000 != 0 {
            self.fighter_int64[(what & 0xFFF) as usize] = val;
        }
        else {
            self.common_int64[(what & 0xFFF) as usize] = val;
        }
    }
    fn _set_float(&mut self, what: i32, val: f32) {
        if what & 0x1000 != 0 {
            self.fighter_float[(what & 0xFFF) as usize] = val;
        }
        else {
            self.common_float[(what & 0xFFF) as usize] = val;
        }
    }
    fn _set_flag(&mut self, what: i32, val: bool) {
        if what & 0x1000 != 0 {
            self.fighter_flag[(what & 0xFFF) as usize] = val;
        }
        else {
            self.common_flag[(what & 0xFFF) as usize] = val;
        }
    }
    fn _countdown_int(&mut self, what: i32, min: i32) -> bool {
        if what & 0x1000 != 0 {
            let what = what & 0xFFF;
            if self.fighter_int[(what & 0xFFF) as usize] <= min { 
                true
            } else {
                self.fighter_int[(what & 0xFFF) as usize] -= 1;
                self.fighter_int[(what & 0xFFF) as usize] <= min
            }
        } else {
            let what = what & 0xFFF;
            if self.common_int[(what & 0xFFF) as usize] <= min { 
                true
            } else {
                self.common_int[(what & 0xFFF) as usize] -= 1;
                self.common_int[(what & 0xFFF) as usize] <= min
            }
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__get_int")]
    pub fn get_int(boma: *mut BattleObjectModuleAccessor, what: i32) -> i32 {
        unsafe {
            get_var_module!(boma)._get_int(what)
        }
    }
    
    #[cfg_attr(feature = "debug", export_name = "VarModule__get_int64")]
    pub fn get_int64(boma: *mut BattleObjectModuleAccessor, what: i32) -> u64 {
        unsafe {
            get_var_module!(boma)._get_int64(what)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__get_float")]
    pub fn get_float(boma: *mut BattleObjectModuleAccessor, what: i32) -> f32 {
        unsafe {
            get_var_module!(boma)._get_float(what)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__is_flag")]
    pub fn is_flag(boma: *mut BattleObjectModuleAccessor, what: i32) -> bool {
        unsafe {
            get_var_module!(boma)._is_flag(what)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__set_int")]
    pub fn set_int(boma: *mut BattleObjectModuleAccessor, what: i32, val: i32) {
        unsafe {
            get_var_module!(boma)._set_int(what, val);
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__set_int64")]
    pub fn set_int64(boma: *mut BattleObjectModuleAccessor, what: i32, val: u64) {
        unsafe {
            get_var_module!(boma)._set_int64(what, val);
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__set_float")]
    pub fn set_float(boma: *mut BattleObjectModuleAccessor, what: i32, val: f32) {
        unsafe {
            get_var_module!(boma)._set_float(what, val);
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__set_flag")]
    pub fn set_flag(boma: *mut BattleObjectModuleAccessor, what: i32, val: bool) {
        unsafe {
            get_var_module!(boma)._set_flag(what, val);
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__on_flag")]
    pub fn on_flag(boma: *mut BattleObjectModuleAccessor, what: i32) {
        Self::set_flag(boma, what, true);
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__off_flag")]
    pub fn off_flag(boma: *mut BattleObjectModuleAccessor, what: i32) {
        Self::set_flag(boma, what, false);
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__countdown_int")]
    pub fn countdown_int(boma: *mut BattleObjectModuleAccessor, what: i32, min: i32) -> bool {
        unsafe {
            get_var_module!(boma)._countdown_int(what, min)
        }
    }

    #[cfg_attr(feature = "debug", export_name = "VarModule__reset")]
    pub fn reset(boma: *mut BattleObjectModuleAccessor, reset_mask: u8) {
        unsafe {
            get_var_module!(boma)._reset(reset_mask);
        }
    }
}