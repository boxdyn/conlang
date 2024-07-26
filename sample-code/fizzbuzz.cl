// FizzBuzz, using the unstable variadic-`print` builtin

fn main() {
    fizzbuzz(10, 20)
}

// Outputs FizzBuzz for numbers between `start` and `end`, inclusive
fn fizzbuzz(start: i128, end: i128) {
    for x in start..=end {
        println(if x % 15 == 0 {
            "FizzBuzz"
        } else if 0 == x % 3 {
            "Fizz"
        } else if x % 5 == 0 {
            "Buzz"
        } else {
            x
        })
    }
}
