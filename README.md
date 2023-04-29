Social bill tracker and civil engagement platform.

# Structure

The project is organized as followed:

* [clients](clients) contains various client implementations for the app.
    - [browser](clients/browser) a browser-based client application, built with React/Typescript.
* [model](model) defines the data model provided by the server. It is a Rust crate.
* [server](server) implements the server application which makes the data queryable for clients.

# Development

## Get the code

```bash
git clone git@github.com:jbearer/bill-tracker.git
cd bill-tracker
```

## Install Nix

This project consists of both a Rust project and a React project, each with their own dependencies.
It uses [Nix](https://nixos.org/) as a one-stop shop for managing the complexity of these
dependencies. While it is possible to develop on this project without Nix, by manually installing
[Cargo](https://doc.rust-lang.org/cargo/), [Node.js](https://nodejs.org/en), and related
dependencies, we highly recommend using Nix to get everything in one shot and be sure you are using
the same versions as other developers.

If you don't already have Nix on your system, install it by following the instructions
[here](https://nixos.org/download.html). Once installed, you can enter a shell with up-to-date
versions of all the necessary dependencies in scope simply by running `nix shell` from the project
root.

Optionally, you can also install [direnv](https://direnv.net/), which will automatically drop you into the Nix
shell whenever you enter the project directory. Use `direnv allow` to enable this convenience after
installing. Once in the Nix shell, you can use `direnv deny` if you ever need to drop out of it.

## Building the Rust projects

The whole project is a Cargo workspace, so to build everything you can simply run

```bash
cargo build --workspace
```

Use `cargo clippy` to run the linter and `cargo test` to run unit tests.

## Building the React client

There is a browser client for the app in `clients/browser`. This is a React Typescript project. To
use it you should first cd in to the directory and make sure the required NPM packages are
installed:

```bash
cd clients/browser
npm install
```

Then you can use various `npm` scripts to develop:
* `npm start` will build a development version of the project, open it in your browser, and watch
  your local directory for code changes
* `npm test` runs unit tests
* `npx eslint --fix` finds and fixes formatting and style issues
