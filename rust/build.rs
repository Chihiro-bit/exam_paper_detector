use std::env;
use std::path::PathBuf;

fn main() {
    // flutter_rust_bridge 代码生成由 flutter_rust_bridge_codegen CLI 工具执行
    // 运行: flutter_rust_bridge_codegen generate
    println!("cargo:rerun-if-changed=src/api.rs");

    // --- Paddle Inference C 库链接 ---
    //
    // 通过环境变量 `PADDLE_INFERENCE_DIR` 定位预编译的 Paddle Inference 库。
    // 该目录应包含:
    //   - paddle/include/paddle_c_api.h
    //   - paddle/lib/paddle_inference_c.{dll,so,dylib}

    let paddle_dir = env::var("PADDLE_INFERENCE_DIR").unwrap_or_else(|_| {
        // 回退: 在 rust/ 目录下查找 paddle_inference 子目录
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let fallback = PathBuf::from(&manifest_dir).join("paddle_inference");
        if fallback.exists() {
            return fallback.to_str().unwrap().to_string();
        }
        panic!(
            "\n\
             ============================================================\n\
             PADDLE_INFERENCE_DIR not set and no paddle_inference/ found.\n\
             \n\
             Please either:\n\
             1. Set PADDLE_INFERENCE_DIR environment variable\n\
             2. Place the library under rust/paddle_inference/\n\
             \n\
             Download:\n\
               python scripts/download_paddle_inference.py\n\
             \n\
             Manual download:\n\
               https://www.paddlepaddle.org.cn/inference/v2.6/guides/install/download_lib.html\n\
             ============================================================\n"
        );
    });

    let paddle_path = PathBuf::from(&paddle_dir);

    // 查找 lib 目录（支持两种目录结构）
    let lib_dir = if paddle_path.join("paddle").join("lib").exists() {
        paddle_path.join("paddle").join("lib")
    } else if paddle_path.join("lib").exists() {
        paddle_path.join("lib")
    } else {
        panic!(
            "Cannot find lib directory in {}.\nExpected paddle/lib/ or lib/",
            paddle_dir
        );
    };

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // 链接 paddle_inference_c（C API 库）
    println!("cargo:rustc-link-lib=dylib=paddle_inference_c");

    // Windows 上还需要链接主库
    #[cfg(target_os = "windows")]
    {
        if lib_dir.join("paddle_inference.lib").exists()
            || lib_dir.join("paddle_inference.dll.lib").exists()
        {
            println!("cargo:rustc-link-lib=dylib=paddle_inference");
        }
    }

    // Linux/macOS: 设置 rpath
    #[cfg(not(target_os = "windows"))]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    println!("cargo:rerun-if-env-changed=PADDLE_INFERENCE_DIR");
    println!("cargo:rerun-if-changed=build.rs");
}
