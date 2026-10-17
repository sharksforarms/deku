# Contributing

## Development

Development commands are defined in the `Justfile` and run in Docker. Run them
from the repository root. Docker and Docker Compose are required.

Build the development image first:

```sh
docker compose build
```

Run the main development checks:

```sh
docker compose run --rm build just build
docker compose run --rm build just test
docker compose run --rm build just ci
docker compose run --rm build just lint
```

`just ci` runs the build, test, example, `no_std`, and WebAssembly checks used
by CI. `just lint` runs formatting and Clippy checks with stable Rust.

The stable toolchain is used by default. Select the MSRV or beta pipeline with
`DEKU_TOOLCHAIN=msrv` or `DEKU_TOOLCHAIN=beta`:

```sh
docker compose run --rm -e DEKU_TOOLCHAIN=msrv build just ci
```

List all available commands with:

```sh
docker compose run --rm build just
```
