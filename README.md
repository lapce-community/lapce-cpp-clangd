# C/C++ (clangd) for Lapce

## Configuration

In `settings.toml`:

```toml
[lapce-cpp-clangd.volt]
clangdVersion = "15.0.0" # git tag from https://github.com/clangd/clangd
serverPath = "<custom clangd>"
serverArgs = [
    #custom args
]
```

## Platforms

Plugin will automatically download `clangd` for below architectures

| Platform | x86_64 (amd64) | aarch64 (arm64) |
| -------- | -------------- | --------------- |
| Linux    | ✔️              | ❌               |
| Windows  | ✔️              | ❌               |
| macOS    | ✔️              | ❌               |
