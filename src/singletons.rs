use std::sync::Once;
use skyline::nn::ro::LookupSymbol;
use super::c_str;

pub static mut BOSS_MANAGER:                   *const *mut smash::app::BossManager                 = 0 as _;
pub static mut ITEM_MANAGER:                   *const *mut smash::app::ItemManager                 = 0 as _;
pub static mut STAGE_MANAGER:                  *const *mut smash::app::StageManager                = 0 as _;
pub static mut FIGHTER_MANAGER:                *const *mut smash::app::FighterManager              = 0 as _;
pub static mut BATTLE_OBJECT_SLOW:             *const *mut smash::app::BattleObjectSlow            = 0 as _;
pub static mut BATTLE_OBJECT_WORLD:            *const *mut smash::app::BattleObjectWorld           = 0 as _;
pub static mut ITEM_PARAM_ACCESSOR:            *const *mut smash::app::ItemParamAccessor           = 0 as _;
pub static mut BATTLE_OBJECT_MANAGER:          *const *mut smash::app::BattleObjectManager         = 0 as _;
pub static mut FIGHTER_CUT_IN_MANAGER:         *const *mut smash::app::FighterCutInManager         = 0 as _;
pub static mut FIGHTER_PARAM_ACCESSOR2:        *const *mut smash::app::FighterParamAccessor2       = 0 as _;
pub static mut GIMMICK_EVENT_PRESENTER:        *const *mut smash::app::GimmickEventPresenter       = 0 as _;
pub static mut FIGHTER_PIT_B_FINAL_MODULE:     *const *mut smash::app::FighterPitBFinalModule      = 0 as _;
pub static mut FIGHTER_BAYONETTA_FINAL_MODULE: *const *mut smash::app::FighterBayonettaFinalModule = 0 as _;
pub static mut LUA_MANAGER:                    *const *mut skyline::libc::c_void                   = 0 as _;
pub static mut EFFECT_MANAGER:                 *const *mut skyline::libc::c_void                   = 0 as _;

static INIT: Once = Once::new();

macro_rules! assign_symbol {
    ($id:ident, $e:expr) => {{
        unsafe {
            let mut sym = 0usize;
            skyline::nn::ro::LookupSymbol(&mut sym as *mut usize, c_str!($e));
            assert!(sym != 0, "Failed to find symbol {}", $e);
            $id = std::mem::transmute(sym);
        }
    }}
}

pub fn init() {
    INIT.call_once(|| {
        assign_symbol!(BOSS_MANAGER,                   "_ZN3lib9SingletonIN3app11BossManagerEE9instance_E");
        assign_symbol!(ITEM_MANAGER,                   "_ZN3lib9SingletonIN3app11ItemManagerEE9instance_E");
        assign_symbol!(STAGE_MANAGER,                  "_ZN3lib9SingletonIN3app12StageManagerEE9instance_E");
        assign_symbol!(FIGHTER_MANAGER,                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E");
        assign_symbol!(BATTLE_OBJECT_SLOW,             "_ZN3lib9SingletonIN3app16BattleObjectSlowEE9instance_E");
        assign_symbol!(BATTLE_OBJECT_WORLD,            "_ZN3lib9SingletonIN3app17BattleObjectWorldEE9instance_E");
        assign_symbol!(ITEM_PARAM_ACCESSOR,            "_ZN3lib9SingletonIN3app17ItemParamAccessorEE9instance_E");
        assign_symbol!(BATTLE_OBJECT_MANAGER,          "_ZN3lib9SingletonIN3app19BattleObjectManagerEE9instance_E");
        assign_symbol!(FIGHTER_CUT_IN_MANAGER,         "_ZN3lib9SingletonIN3app19FighterCutInManagerEE9instance_E");
        assign_symbol!(FIGHTER_PARAM_ACCESSOR2,        "_ZN3lib9SingletonIN3app21FighterParamAccessor2EE9instance_E");
        assign_symbol!(GIMMICK_EVENT_PRESENTER,        "_ZN3lib9SingletonIN3app21GimmickEventPresenterEE9instance_E");
        assign_symbol!(FIGHTER_PIT_B_FINAL_MODULE,     "_ZN3lib9SingletonIN3app22FighterPitBFinalModuleEE9instance_E");
        assign_symbol!(FIGHTER_BAYONETTA_FINAL_MODULE, "_ZN3lib9SingletonIN3app27FighterBayonettaFinalModuleEE9instance_E");
        assign_symbol!(LUA_MANAGER,                    "_ZN3lib9SingletonINS_10LuaManagerEE9instance_E");
        assign_symbol!(EFFECT_MANAGER,                 "_ZN3lib9SingletonINS_13EffectManagerEE9instance_E");
    });
}

macro_rules! expose_singleton {
    ($($public:ident, $private:ident)*) => {
        $(
            #[macro_export]
            macro_rules! $public {
                () => { *$private }
            }
        )*
    }
}

expose_singleton!(
    BossManager,                 BOSS_MANAGER
    ItemManager,                 ITEM_MANAGER
    StageManager,                STAGE_MANAGER
    FighterManager,              FIGHTER_MANAGER
    BattleObjectSlow,            BATTLE_OBJECT_SLOW
    BattleObjectWorld,           BATTLE_OBJECT_WORLD
    ItemParamAccessor,           ITEM_PARAM_ACCESSOR
    FighterCutInManager,         FIGHTER_CUT_IN_MANAGER
    FighterParamAccessor2,       FIGHTER_PARAM_ACCESSOR2
    GimmickEventPresenter,       GIMMICK_EVENT_PRESENTER
    FighterPitBFinalModule,      FIGHTER_PIT_B_FINAL_MODULE
    FighterBayonettaFinalModule, FIGHTER_BAYONETTA_FINAL_MODULE
    LuaManager,                  LUA_MANAGER
    EffectManager,               EFFECT_MANAGER
);