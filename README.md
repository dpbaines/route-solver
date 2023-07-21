# route-solver
[![Rust](https://github.com/dpbaines/route-solver/actions/workflows/rust.yml/badge.svg)](https://github.com/dpbaines/route-solver/actions/workflows/rust.yml)

A route optimizer which aims to make multi-stop trip planning easier. By scraping combinations of flight timing and ordering the goal is make flying cheaper by choosing the best flight itineraries automatically. This is a fullstack project which aims to use Rust on both sides of the stack. 

Uses the SkyScanner API for now.

## Technologies
1. yew
2. actix-web
3. reqwest
