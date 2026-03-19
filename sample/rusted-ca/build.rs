fn main() {
    // protoファイルからRustコードを生成
    prost_build::compile_protos(&["proto/hello.proto"], &["proto"]).unwrap();

    // プロトファイルが変更されたときに再ビルドを促す
    println!("cargo:rerun-if-changed=proto/hello.proto");
}
