//! This is a Conlang library demonstrating `match`

struct Student {
    name: str,
    age: i32,
}

fn match_test(student: Student) {
    match student {
        Student { name: "shark", age } => println("Found a shark of ", age, " year(s)");
        Student { name, age: 22 } => println("Found a 22-year-old named ", name);
        Student { name, age: age if age < 18 } => println("Found a minor named ", name);
        Student { name, age if age >= 65 } => println("Found a senior aged ", age, " named ", name);
        Student { name, age } => println("Found someone named ", name, " of ", age, " year(s)");
    }
}

fn main() {
    /// Found a shark of 22 year(s)
    match_test(Student { name: "shark", age: 22 });
    /// Found a 22-year-old named name22
    match_test(Student { name: "name22", age: 22 });
    /// Found a minor named name10
    match_test(Student { name: "name10", age: 10 });
    /// Found a senior aged 69 named name69
    match_test(Student { name: "name69", age: 69 });
    /// Found someone named name50 of 50 year(s)
    match_test(Student { name: "name50", age: 50 });
}
