//! This is a Conlang library demonstrating `match`

struct Student {
    name: str,
    age: i32,
}

fn Student(name: str, age: i32) -> Student {
    Student { name, age }
}

fn match_test(student: Student) {
    match student {
        Student { name: "shark", age } => println("Found a shark of ", age, " year(s)"),
        Student { name, age: 22 } => println("Found a 22-year-old named ", name),
        Student { name, age } => println("Found someone named ", name, " of ", age, " year(s)"),
    }
}
