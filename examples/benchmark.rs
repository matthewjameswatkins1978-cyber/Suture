fn main() {
    std::process::exit(threadmoth::benchmark::run(
        threadmoth::benchmark::Profile::Standard,
        false,
    ));
}
