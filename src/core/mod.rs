//! 跨层共享类型：输入层产生的「消息」（[`Chip`]）与跨线程「控制状态」
//! （[`PauseControl`]），被 input / overlay / app / main 各层共用。
//!
//! 它们不属于某一具体层，因此独立成模块，避免各层互相引用彼此的私有细节。

pub mod chip;
pub mod control;

pub use chip::{Accent, Chip};
pub use control::PauseControl;
