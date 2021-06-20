# BULLHORN

_"Turning blogging up to 11 since 1849"_

CLI tool to publish articles and update them.

## Usage

```sh
cargo install cargo_bullhorn
# Assuming ~/.cargo/bin is in `PATH` environment variable
cargo_bullhorn --help
```


```sh
cargo_bullhorn 0.1.0

USAGE:
    cargo_bullhorn [FLAGS] [OPTIONS] [posts]...

ARGS:
    <posts>...    One or more markdown files to post

FLAGS:
        --dry        Dry run (e.g. no REST POST/PUT, GraphQL mutation, etc.)
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --compare <compare>
            How articles are compared to determine if they already exist for update [default:
            canonical-url] [possible values: canonical-url]

        --config <config>                                      [default: $HOME/.rollout.yaml]
        --date <date>                                          Publish date if not today
        --devto-api-token <devto-api-token>                    
        --hashnode-api-token <hashnode-api-token>              
        --hashnode-publication-id <hashnode-publication-id>    
        --medium-api-token <medium-api-token>                  
        --medium-publication-id <medium-publication-id>        
        --operation <operation>
            Operation to perform (i.e. update, or submit new) [default: auto] [possible values:
            auto, put, post]

        --remote <remote>                                      Git remote to use [default: origin]
```

## Features

- Supports [Jekyll front-matter](https://jekyllrb.com/docs/front-matter/)

| | Github Pages | [Medium](https://medium.com/) | [hashnode](https://hashnode.com/) | [dev.to](https://dev.to/)
|-|-|-|-|-
| Canonical source | ✅
| Cross-post Articles | | ✅ | ✅ | ✅
| Update articles | | 🚫 | 👎 | ✅
| Front-matter Tags | ✅ | ✅ | 👎 | ✅
| Front-matter Date | | 🚫 | 👎 | 👎
| Publications | | 👎 | ✅ | 🚫

🚫 = Not supported
👎 = _Might_ work.  Has [issues](https://github.com/jeikabu/cargo_bullhorn/issues).
