#!/usr/bin/env -S conlang main()

fn link (link, text) -> &str
    "\x1b]8;;" + link + "\x1b\\" + text + "\x1b]8;;\x1b\\";

fn main () {
    println(link("https://conlang.foo/docs", "Here's a link to the docs!"));
    println(link("https://conlang.foo/source", "Here's a link to the source!"))
}
