use serde_derive::Serialize;

#[derive(Serialize)]
struct Person {
    name: String,
    height: f64,
    adult: bool,
    children: Vec<Person>,
}

fn main() {
    let person = Person {
        name: String::from("Anne Mustermann"),
        height: 1.76f64,
        adult: true,
        children: vec![Person {
            name: String::from("Max Mustermann"),
            height: 1.32f64,
            adult: false,
            children: vec![],
        }],
    };

    let bytes = serde_yaml::to_vec(&person).unwrap();
    bat::PrettyPrinter::new()
        .language("yaml")
        .line_numbers(true)
        .grid(true)
        .header(true)
        .input(
            bat::Input::from_bytes(&bytes)
                // .name("person.yaml")
                // .kind("File"),
        )
        .print()
        .unwrap();
}
