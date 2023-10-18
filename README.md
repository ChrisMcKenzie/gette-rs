# Gette-rs: Rust Downloader Library

![Rust Version](https://img.shields.io/badge/Rust-1.73%2B-green.svg)
![rust](https://github.com/ChrisMcKenzie/gette-rs/actions/workflows/rust.yml/badge.svg)

Gette-rs is a versatile and high-performance Rust library designed for downloading files from various sources, including local files and cloud blob stores. This library is intended for developers who need a reliable and efficient way to fetch data from a wide range of sources while maintaining Rust's safety and performance standards.

## Features

- **Source Agnostic**: Gette-rs supports multiple sources, including local files, Amazon S3, Azure Blob Storage, Google Cloud Storage, GIT, and HTTP/HTTPS URLs.

- **Asynchronous**: Take full advantage of Rust's asynchronous capabilities for concurrent and non-blocking operations.

- **Error Handling**: Robust error handling to ensure the integrity of your downloads.

- **Configurability**: Fine-tune your download settings with various options, such as retries, timeouts, and more.

- **Rust Ecosystem Compatibility**: Easily integrate Gette-rs with other libraries, frameworks, and tools within the Rust ecosystem.

## Getting Started

Add Gette-rs to your project's `Cargo.toml`:

```toml
[dependencies]
gette = "0.1"
```


### Basic Usage

Downloading a file is straightforward with Gette-rs:

```rust
use gette::Builder
use gette::getters;

fn main()  {
    let dest = "/tmp/readme.md";
    let source = "git://github.com/chrismckenzie/gette-rs/readme.md";
    let builder = Builder::new();
    builder.add_getter(git::Getter::new());
    builder.get(dest, source).unwrap();
    println!("File downloaded successfully!");
    Ok(())
}
```

For more advanced usage, including cloud storage integration, please refer to the [official documentation](https://github.com/your/repo/link).

## Contributing

Gette-rs is an open-source project, and we welcome contributions from the community. If you find a bug, have a feature request, or want to contribute code, please refer to our [Contribution Guidelines](https://github.com/your/repo/link).

## Contact

If you have questions or need assistance, feel free to contact us via [email](mailto:chris@chrismckenzie.io) or [open an issue](https://github.com/ChrisMcKenzie/gette-rs/issues).

---

Thank you for choosing Gette-rs! We hope this library serves you well in your Rust project. Your feedback and contributions are highly appreciated.
