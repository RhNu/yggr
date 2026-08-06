//! 材质系统:PNG 安全校验、存储、textures 属性生成与签名
//!
//! 安全要求(authlib-injector 规范):
//! - 先读图像头获取尺寸,防止 PNG bomb 消耗内存
//! - 校验尺寸:皮肤为 64x32/64x64 整数倍;披风为 64x32/22x17 整数倍(22x17 需补足)
//! - 重新编码 PNG 以去除与位图无关的数据(防隐藏恶意代码)

mod payload;
mod process;
mod store;

pub use payload::{build_textures_value, sign_textures_value};
pub use process::{pad_cape, sanitize_png};
pub use store::{TextureStore, import_texture_file};
