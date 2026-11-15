# Windows development

Build and run Vults from a checkout on Windows. Linux keeps its own setup in
[Getting started](../getting-started.md#install).

Install these prerequisites together: Node.js 22 or a newer supported LTS, Rust for
`x86_64-pc-windows-msvc`, Visual Studio **Desktop development with C++** (MSVC x64/x86,
Windows SDK and CMake/Ninja tools), LLVM with `libclang.dll`, and Microsoft Edge WebView2 Runtime.
Set `LIBCLANG_PATH` to LLVM's `bin` directory if it is installed elsewhere. Visual Studio Insiders
is supported by the launcher; Ninja avoids its CMake instance-discovery issues.

From the repository root:

```bat
scripts\dev-windows.cmd --check
npm ci
scripts\dev-windows.cmd
```

The check reports missing tools before compiling. The launcher changes only its own environment.
If a previous build cached a different CMake generator, run `cargo clean -p whisper-rs-sys` once.

The launcher enables the `voice-vulkan` build feature when Vulkan SDK headers, `glslc` and
`vulkan-1.lib` are available through `VULKAN_SDK`, `C:\VulkanSDK`, or `target/local-tools/vulkan`.
Use `scripts\dev-windows.cmd --gpu` to require that build, or `--cpu` for the CPU fallback.
The SDK is a build dependency; execution uses the GPU vendor's Vulkan driver. The launcher reports
the chosen build before starting. GPU builds temporarily map an unused drive from V through Z to
their checkout-local build cache to avoid MSVC shader-generator path limits, then remove that mapping
on normal launcher exit. An explicit `CARGO_TARGET_DIR` keeps its existing owner and path.
The first GPU compilation can take several minutes; this is separate from transcription latency.

Build the Windows installer with `npm run bundle:windows -- --bundles nsis`. This command always
enables Vulkan; a missing SDK never silently produces a CPU-only installer. It prepares pinned,
SHA-256-checked SDK tools and an x64 Vulkan loader locally, without running their installers or
changing system drivers. The loader and its license ship with Vults, so end users do not need the SDK.
The app uses CPU transcription when no compatible Vulkan device is available.
Packaging extracts the NSIS payload, checks its app/hook/loader/license against the build, and probes
the packaged executable both normally and with driver discovery disabled. The probe
(`vults.exe --voice-backend-check`) exits before profiles, windows or microphones start.
The verified installer is copied to `target/release/bundle/nsis`, including when the build cache is
on another drive. Release CI uses the same command and records the verification report.
