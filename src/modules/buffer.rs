use smash::app::BattleObjectModuleAccessor;
use super::BUFFER_MODULE_OFFSET;

macro_rules! get_buffer_module {
    ($boma:ident) => {{
        let vtable = *($boma as *const *const u64);
        &mut *(*vtable.offset((*vtable.offset(-1) as isize) + BUFFER_MODULE_OFFSET) as *mut BufferModule)
    }}
}

macro_rules! has_buffer_module {
    ($boma:ident) => {{
        let vtable = *($boma as *const *const u64);
        super::is_hdr_boma($boma) && !(*vtable.offset((*vtable.offset(-1) as isize) + BUFFER_MODULE_OFFSET) as *mut BufferModule).is_null()
    }}
}

pub struct CommandFlag {
    pub on_last_frame: u32,
    pub should_hold: [bool; 32],
    pub hold_frame: [i32; 32],
    pub hold_frame_max: [i32; 32]
}

impl CommandFlag {
    pub fn new() -> Self {
        Self {
            on_last_frame: 0,
            should_hold: [false; 32],
            hold_frame: [0; 32],
            hold_frame_max: [-1; 32]
        }
    }

    pub fn clear(&mut self) {
        self.on_last_frame = 0;
        self.should_hold = [false; 32];
        self.hold_frame = [0; 32];
        self.hold_frame_max = [-1; 32]
    }

    pub fn update(
        &mut self,
        game_held: &mut [u8],
        max_hold_frame: i32,
        press_frame: i32,
        should_hold: bool
    ) {
        self.on_last_frame = 0;
        for (idx, x) in game_held.iter_mut().enumerate() {
            if *x != 0
            && (self.hold_frame[idx] < press_frame || self.should_hold[idx] || should_hold || *x != 1) {
                self.hold_frame[idx] += 1;
                println!("{:#x} | {:#x}", self.hold_frame[idx], press_frame);
                if self.hold_frame[idx] < press_frame {
                    continue;
                }
                if *x == 1 {
                    if self.should_hold[idx] {
                        if self.hold_frame_max[idx] != -1 && self.hold_frame_max[idx] < self.hold_frame[idx] {
                            *x = 0;
                            self.hold_frame[idx] = 0;
                            continue;
                        }
                    } else if should_hold {
                        if max_hold_frame != -1 && max_hold_frame < self.hold_frame[idx] {
                            *x = 0;
                            self.hold_frame[idx] = 0;
                            continue;
                        }
                    }
                }
                self.on_last_frame |= 1 << idx;
            } else {
                self.hold_frame[idx] = 0;
                *x = 0;
            }
        }
    }
}

pub struct BufferModule {
    pub owner: *mut BattleObjectModuleAccessor,
    pub cats: [CommandFlag; 4],
    pub hold_all: bool,
    pub hold_all_frame_max: i32
}

impl BufferModule {
    pub fn new(owner: *mut BattleObjectModuleAccessor) -> Self {
        Self {
            owner,
            cats: [
                CommandFlag::new(),
                CommandFlag::new(),
                CommandFlag::new(),
                CommandFlag::new()
            ],
            hold_all: false,
            hold_all_frame_max: -1
        }
    }

    fn _persist_command_one(&mut self, category: i32, flag: i32) {
        let flag = flag & 0x1F;
        self.cats[category as usize].should_hold[flag as usize] = true;
        // self.cats[category as usize].hold_frame[flag as usize] = 0; // uncomment this line to reset the hold frame to 0 upon setting persist_command_one
        self.cats[category as usize].hold_frame_max[flag as usize] = -1;
    }

    fn _persist_command_one_with_lifetime(&mut self, category: i32, flag: i32, lifetime: i32) {
        self._persist_command_one(category, flag);
        self.cats[category as usize].hold_frame_max[(flag & 0x1F) as usize] = lifetime;
    }

    fn _set_persist_lifetime(&mut self, lifetime: i32) {
        self.hold_all_frame_max = lifetime;
    }

    fn _enable_persist(&mut self) {
        self.hold_all = true;
    }

    fn _unable_persist(&mut self) {
        self.hold_all = false;
    }

    fn _clear_persist(&mut self) {
        self.cats[0].clear();
        self.cats[1].clear();
        self.cats[2].clear();
        self.cats[3].clear();
    }

    fn _clear_persist_one(&mut self, category: i32, flag: i32) {
        let cat = &mut self.cats[category as usize];
        cat.on_last_frame &= !(1 << (flag as usize));
        cat.should_hold[flag as usize] = false;
        cat.hold_frame[flag as usize] = 0;
        cat.hold_frame_max[flag as usize] = -1;
    }

    fn _exec(&mut self, cats: &mut [&mut [u8]; 4]) {
        let press_frame = unsafe {
            smash::app::lua_bind::ControlModule::get_command_life_count_max(self.owner) as i32
        };
        for x in 0..4 {
            self.cats[x].update(cats[x], self.hold_all_frame_max, press_frame - 1, self.hold_all);
        }
    }

    fn _is_persist(&self) -> bool {
        self.hold_all
    }

    fn _is_persist_one(&self, category: i32, flag: i32) -> bool {
        self.cats[category as usize].should_hold[flag as usize]
    }

    fn _persist_lifetime(&self) -> i32 {
        self.hold_all_frame_max
    }

    fn _persist_lifetime_one(&self, category: i32, flag: i32) -> i32 {
        self.cats[category as usize].hold_frame[flag as usize]
    }

    fn _persist_lifetime_max_one(&self, category: i32, flag: i32) -> i32 {
        self.cats[category as usize].hold_frame_max[flag as usize]
    }

    /// Enables the hold buffer for one input only
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to enable hold buffer for
    pub fn persist_command_one(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32) {
        unsafe {
            get_buffer_module!(boma)._persist_command_one(category, flag)
        }
    }

    /// Enables hold buffer for one input only with a maximum number of frames to hold it
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to enable hold buffer for
    /// * `lifetime` - The number of frames which you can hold it before it is cleared
    pub fn persist_command_one_with_lifetime(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32, lifetime: i32) {
        unsafe {
            get_buffer_module!(boma)._persist_command_one_with_lifetime(category, flag, lifetime)
        }
    }

    /// Sets the maximum hold buffer frames for when it is enabled globally
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `lifetime` - The number of frames for which hold buffer should be allowed
    /// ## Note
    /// If an input is being held, this value is not considered if the value has it's own lifetime set with `persist_command_one_with_lifetime`
    pub fn set_persist_lifetime(boma: *mut BattleObjectModuleAccessor, lifetime: i32) {
        unsafe {
            get_buffer_module!(boma)._set_persist_lifetime(lifetime)
        }
    }

    /// Enables hold buffer for the agen
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    pub fn enable_persist(boma: *mut BattleObjectModuleAccessor) {
        unsafe {
            get_buffer_module!(boma)._enable_persist()
        }
    }

    /// Disables hold buffer for the agent
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    pub fn unable_persist(boma: *mut BattleObjectModuleAccessor) {
        unsafe {
            get_buffer_module!(boma)._unable_persist()
        }
    }

    /// Clears all hold buffer inputs for the agent, and sets their hold lifetimes to 0, and disables hold buffering them
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    pub fn clear_persist(boma: *mut BattleObjectModuleAccessor) {
        unsafe {
            get_buffer_module!(boma)._clear_persist()
        }
    }

    /// Clears the hold buffer input for the agent, and sets their hold lifetim to 0, and disables hold buffering them
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to clear the hold for
    pub fn clear_persist_one(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32) {
        unsafe {
            get_buffer_module!(boma)._clear_persist_one(category, flag)
        }
    }

    /// Updates the hold buffer
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `cats` - An array of 4 slices of the different categories
    pub fn exec(boma: *mut BattleObjectModuleAccessor, cats: &mut [&mut [u8]; 4]) {
        unsafe {
            get_buffer_module!(boma)._exec(cats)
        }
    }

    /// Checks if hold buffer is enabled on this agent (general)
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// # Returns
    /// Whether or not the hold buffer is enabled generally
    pub fn is_persist(boma: *mut BattleObjectModuleAccessor) -> bool {
        unsafe {
            get_buffer_module!(boma)._is_persist()
        }
    }
    
    /// Checks if hold buffer is enabled on this agent for the specified input
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to check hold buffer for
    /// # Returns
    /// Whether or not the hold buffer is enabled for this input
    pub fn is_persist_one(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32) -> bool {
        unsafe {
            get_buffer_module!(boma)._is_persist_one(category, flag)
        }
    }
    
    /// Gets the general hold buffer frame max on this agent
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// # Returns
    /// The number of frames hold buffer will exist for
    pub fn persist_lifetime(boma: *mut BattleObjectModuleAccessor) -> i32 {
        unsafe {
            get_buffer_module!(boma)._persist_lifetime()
        }
    }
    
    /// Gets the current lifetime of a specific input in the hold buffer (including tap buffer frames)
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to get the lifetime for
    /// # Returns
    /// The number of frames the input has been held
    pub fn persist_lifetime_one(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32) -> i32 {
        unsafe {
            get_buffer_module!(boma)._persist_lifetime_one(category, flag)
        }
    }
    
    /// Gets the maximum lifetime for a specific input in the hold buffer
    /// # Arguments
    /// * `boma` - The module accessor for this agent
    /// * `category` - The command flag category
    /// * `flag` - The specific flag to get the lifetime for
    /// # Returns
    /// The maximum number of frames the input can be held before getting cleared
    pub fn persist_lifetime_max_one(boma: *mut BattleObjectModuleAccessor, category: i32, flag: i32) -> i32 {
        unsafe {
            get_buffer_module!(boma)._persist_lifetime_max_one(category, flag)
        }
    }
}


#[repr(C)]
#[derive(Debug)]
struct CommandFlagCat {
    flags: u32,
    unk4: u32,
    count: usize,
    lifetimes: *mut u8,
    lifetimes2: *mut u8,
    lifetimes3: *mut u64
}

impl CommandFlagCat {
    fn lifetimes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.lifetimes, self.count)
        }
    }

    fn lifetimes_mut(&self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.lifetimes, self.count)
        }
    }

    fn lifetimes2(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.lifetimes2, self.count)
        }
    }
}

#[skyline::hook(offset = 0x6ba980)]
unsafe fn get_command_flag_cat_replace(control_module: u64, cat: i32) -> u32 {
    // println!("test");
    let cats = std::slice::from_raw_parts((control_module + 0x568) as *const CommandFlagCat, 4);
    let mut output = 0;
    let lifetimes = cats[cat as usize].lifetimes();
    let lifetimes2 = cats[cat as usize].lifetimes2();
    for x in 0..cats[cat as usize].count {
        if lifetimes[x] > 0  && lifetimes2[x] <= 1 {
            output |= 1 << x;
        }
    }
    output
}

#[skyline::hook(offset = 0x6babf0)]
unsafe fn exec_command(control_module: u64, flag: bool) {
    original!()(control_module, flag);
    let mut cats = std::slice::from_raw_parts_mut((control_module + 0x568) as *mut CommandFlagCat, 4);
    let mut lifetimes = [
        cats[0].lifetimes_mut(),
        cats[1].lifetimes_mut(),
        cats[2].lifetimes_mut(),
        cats[3].lifetimes_mut(),
    ];
    let boma = *((control_module + 0x8) as *mut *mut BattleObjectModuleAccessor);
    if has_buffer_module!(boma) {
        BufferModule::exec(boma, &mut lifetimes);
    }
}

pub fn init() {
    skyline::install_hooks!(
        get_command_flag_cat_replace,
        exec_command
    );
}