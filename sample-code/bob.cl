#!/usr/bin/env -S conlang bob()

/// Prints the lyrics to "ninety-nine bottles of beer"
fn bob() = for bottle in 1..100 verse(bottle);

/// prints a verse of the song
fn verse(count) = println(
    bottles(100 - count), " of beer on the wall\n",
    bottles(100 - count), " of beer!\n",
    "Take one down\nPass it around\n",
    bottles(99 - count), " of beer on the wall!\n"
);

/// Determines the bottle arity
fn bottles(remaining) = match remaining {
    1 => "1 bottle";
    _ => fmt(remaining, " bottles");
};
