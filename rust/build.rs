fn main() {
    // flutter_rust_bridge 代码生成由 flutter_rust_bridge_codegen CLI 工具执行
    // build.rs 中不需要额外操作
    // 运行: flutter_rust_bridge_codegen generate
    println!("cargo:rerun-if-changed=src/api.rs");
}
