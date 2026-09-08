//! 程序配置：位置、大小、显示时长、主题等。
//!
//! 目前以常量形式集中在 [`settings`] 中管理；后续可改为读取 TOML 配置文件
//! （`serde`），对应 md 计划里的「后期」阶段。

pub mod settings;

pub use settings::*;
