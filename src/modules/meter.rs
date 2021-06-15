use smash::app::BattleObjectModuleAccessor;
use super::METER_MODULE_OFFSET;

macro_rules! get_meter_module {
    ($boma:ident) => {{
        let vtable = *($boma as *const *const u64);
        &mut *(*vtable.offset((*vtable.offset(-1) as isize) + METER_MODULE_OFFSET) as *mut MeterModule)
    }}
}

pub struct MeterModule {
    internal_count: i32,
    level: i32,
    to_gain: i32,
    level_up_count: i32,
    max_level: i32
}

impl MeterModule {
    unsafe fn _add_level(&mut self, amount: i32) {
        self.level = (self.level + amount).clamp(0, self.max_level);
        self.internal_count = (self.internal_count + amount * self.internal_count).clamp(0, self.max_level * self.level_up_count);
    }

    unsafe fn _sub_level(&mut self, amount: i32) {
        self.level = (self.level - amount).clamp(0, self.max_level);
        self.internal_count = (self.internal_count - amount * self.level_up_count).clamp(0, self.max_level * self.level_up_count);
    }

    unsafe fn _set_level(&mut self, level: i32) {
        self.level = level.clamp(0, self.max_level);
        self.internal_count = self.level_up_count * level;
    }

    unsafe fn _add_count(&mut self, amount: i32) {
        self.internal_count = (self.internal_count + amount).clamp(0, self.max_level * self.level_up_count);
        self.level = self.internal_count / self.level_up_count;
    }

    unsafe fn _sub_count(&mut self, amount: i32) {
        self.internal_count = (self.internal_count - amount).clamp(0, self.max_level * self.level_up_count);
        self.level = self.internal_count / self.level_up_count;
    }

    unsafe fn _set_count(&mut self, amount: i32) {
        self.internal_count = amount.clamp(0, self.max_level * self.level_up_count);
        self.level = self.internal_count / self.level_up_count;
    }

    unsafe fn _set_potential(&mut self, amount: i32) {
        self.to_gain = amount;
    }

    unsafe fn _clear_potential(&mut self) {
        self.to_gain = 0;
    }

    unsafe fn _use_potential(&mut self) {
        self._add_count(self.to_gain);
        self.to_gain = 0;
    }

    unsafe fn _use_levels(&mut self, amount: i32) -> bool {
        if self.level > amount {
            self.level -= amount;
            true
        } else {
            false
        }
    }

    pub(crate) fn new(level_up_count: i32, max_level: i32) -> Self {
        Self {
            internal_count: 0,
            level: 0,
            to_gain: 0,
            level_up_count,
            max_level
        }
    }

    #[export_name = "MeterModule__add_level"]
    pub fn add_level(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._add_level(amount)
        }
    }

    #[export_name = "MeterModule__sub_level"]
    pub fn sub_level(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._sub_level(amount)
        }
    }

    #[export_name = "MeterModule__set_level"]
    pub fn set_level(module_accessor: *mut BattleObjectModuleAccessor, level: i32) {
        unsafe {
            get_meter_module!(module_accessor)._set_level(level)
        }
    }

    #[export_name = "MeterModule__add_meter"]
    pub fn add_meter(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._add_count(amount)
        }
    }

    #[export_name = "MeterModule__drain_meter"]
    pub fn drain_meter(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._sub_count(amount)
        }
    }

    #[export_name = "MeterModule__set_meter"]
    pub fn set_meter(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._set_count(amount)
        }
    }

    #[export_name = "MeterModule__set_potential"]
    pub fn set_potential(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) {
        unsafe {
            get_meter_module!(module_accessor)._set_potential(amount)
        }
    }

    #[export_name = "MeterModule__clear_potential"]
    pub fn clear_potential(module_accessor: *mut BattleObjectModuleAccessor) {
        unsafe {
            get_meter_module!(module_accessor)._clear_potential()
        }
    }

    #[export_name = "MeterModule__use_potential"]
    pub fn use_potential(module_accessor: *mut BattleObjectModuleAccessor) {
        unsafe {
            get_meter_module!(module_accessor)._use_potential()
        }
    }

    #[export_name = "MeterModule__use_levels"]
    pub fn use_levels(module_accessor: *mut BattleObjectModuleAccessor, amount: i32) -> bool {
        unsafe {
            get_meter_module!(module_accessor)._use_levels(amount)
        }
    }

}