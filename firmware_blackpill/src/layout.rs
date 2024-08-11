use keyberon::action::{k, Action::*, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

const TIMEOUT: u16 = 200;

const SHIFT_SP: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LShift),
    tap: k(Space),
});

const CTRL_TAB: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LCtrl),
    tap: k(Tab),
});

const ALT_ENT: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LAlt),
    tap: k(Enter),
});

const PPN: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(MediaNextSong),
    tap: k(MediaPlayPause),
});

const LBR_FNLAYER: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: Layer(1),
    tap: k(LBracket),
});
const RBR_FNLAYER: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: Layer(1),
    tap: k(RBracket),
});

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<12, 4, 2, ()> = keyberon::layout::layout! {
    {
        [{LBR_FNLAYER} Q     W     E     R     T     Y     U     I     O     P       {RBR_FNLAYER}],
        [CapsLock      A     S     D     F     G     H     J     K     L     SColon  CapsLock],
        [LGui          Z     X     C     V     B     N     M     ,     .     Slash   LGui],
        [ t     t  t NonUsBslash LShift {CTRL_TAB} {ALT_ENT} {SHIFT_SP} NonUsBslash t  t      t   ],
    }{
        [n  {Custom(())}  n     n     VolUp    n   F12  F7  F8  F9  {Custom(())}  n],
        [t  n             n     n     {PPN}    n   F11  F4  F5  F6  n  t],
        [n  n             n     n     VolDown  n   F10  F1  F2  F3  n  n],
        [t  t             t     t     t        t   t    t   t   t   t  t],
    }};
