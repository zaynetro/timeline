use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/sync.proto"], &["proto/"])?;

    // protobuf_codegen::Codegen::new()
    //     // Use `protoc` parser, optional.
    //     .protoc()
    //     // Use `protoc-bin-vendored` bundled protoc command, optional.
    //     .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
    //     // All inputs and imports from the inputs must reside in `includes` directories.
    //     .includes(&["proto"])
    //     // Inputs must reside in some of include paths.
    //     .input("proto/timeline.proto")
    //     .input("proto/sync.proto")
    //     // Specify output directory relative to Cargo output directory.
    //     .cargo_out_dir("protos2")
    //     .run_from_script();
    Ok(())
}
