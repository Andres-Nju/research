fn main() {
    let input = io::stdin();
    let mut input = input.lock();

    let mut map = HashMap::new();
    //~^ ERROR E0433
    //~| NOTE Use of undeclared type or module `HashMap`

    for line in input.lines() {
        let line = line.unwrap();
        println!("process: {}", line);
        let mut parts = line.splitn(2, ":");
        let _logfile = parts.next().unwrap();
        let rest = parts.next().unwrap();
        let mut parts = line.split(" [-] ");

        let stamp = parts.next().unwrap();

        let rest = parts.next().unwrap();
        let words = rest.split_whitespace().collect::<Vec<_>>();

        let instance = words.iter().find(|a| a.starts_with("i-")).unwrap();
        let name = words[1].to_owned();
        let mut entry = map.entry(instance.to_owned()).or_insert(Instance {
            name: name,
            start: None,
            end: None,
        });

        if rest.contains("terminating") {
            assert!(entry.end.is_none());
            entry.end = Some(stamp.to_string());
        }
        if rest.contains("waiting for") {
            assert!(entry.start.is_none());
            entry.start = Some(stamp.to_string());
        }

    }

    println!("{:?}", map);
}
