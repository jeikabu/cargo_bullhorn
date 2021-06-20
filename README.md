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
cargo_bullhorn 0.2.0

USAGE:
    cargo_bullhorn [FLAGS] [OPTIONS] [--] [posts]...

ARGS:
    <posts>...    One or more markdown files

FLAGS:
        --draft      Posts created as drafts, if possible
        --dry        Dry run (e.g. no REST POST/PUT, GraphQL mutation, etc.)
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --compare <compare>
            How articles are compared to determine if they already exist for update [default:
            canonical-url] [possible values: canonical-url, slug]

        --config <config>
            YAML file containing configuration [default: $HOME/.bullhorn.yaml]

        --date <date>                                      Publish date if not today
        --devto-api-token <devto-api-token>                [env: DEVTO_API_TOKEN=]
        --hashnode-api-token <hashnode-api-token>          [env: HASHNODE_USERNAME=]
        --hashnode-username <hashnode-username>            [env: HASHNODE_API_TOKEN=]
        --medium-api-token <medium-api-token>              [env: MEDIUM_API_TOKEN=]
        --medium-publication-id <medium-publication-id>    [env: MEDIUM_PUBLICATION_ID=]
        --operation <operation>
            Operation to perform (i.e. update, or submit new) [default: auto] [possible values:
            auto, put, post]

        --platforms <platforms>...
            Platform(s) to enable [default: medium devto hashnode] [possible values: medium, devto,
            hashnode]

        --remote <remote>                                  Git remote to use [default: origin]
        --slug <slug>                                      Override front-matter `slug` value
        --update-fields <update-fields>...
            Article fields to write when updating an article [possible values: body, slug, tags]
```

## Features

- Front-matter support:
    - [Jekyll](https://jekyllrb.com/docs/front-matter/)
    - [Hugo](https://gohugo.io/content-management/front-matter/) (_some_: `slug`, `series`)


| | Github Pages | [Medium](https://medium.com/) | [hashnode](https://hashnode.com/) | [dev.to](https://dev.to/)
|-|-|-|-|-
| Canonical source | ✅
| Cross-post Articles | | ✅ | ✅ | ✅
| Update articles | | 🚫 | 👎 | ✅
| Front-matter Tags | ✅ | ✅ | 👎 | ✅
| Front-matter Date | | 🚫 | ✅ | ✅
| Publications | | 👎 | ✅ | 🚫

🚫 = Not supported
👎 = _Might_ work.  Has [issues](https://github.com/jeikabu/cargo_bullhorn/issues).
