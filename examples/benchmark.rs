fn main() {
    std::process::exit(suture::benchmark::run(
        suture::benchmark::Profile::Standard,
        false,
    ));
}
