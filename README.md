# Has Marty Zapped Today?

Simple site for determining if [Marty Bent](https://njump.me/npub1guh5grefa7vkay4ps6udxg8lrqxg2kgr3qh9n4gduxut64nfxq0q9y6hjy) has zapped yet today.

## Development

Ensure Rust and Cargo are installed. The easiey way to do that is using [rustup](https://rustup.rs/). Then run the development server and open up `localhost:8000`.

```shell
cargo run
```

## Building and Deployment

To build the application, use `cargo` like normal. When deploying, the expectation is the `assets` directory is also available. This can be made simpler by running the available build script.

```shell
./bin/build.sh
```
