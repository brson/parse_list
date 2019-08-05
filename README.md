# parse_list for Rust

Parse files and lists of stringified things into lists of thingified things.

That is, if you've got something `Read`-y or `Iterator`-y over `u8`s,
`String`s, or `str`s, and a type that implements `FromStr`, you can also
have an `Iterator` of that type of thing.

Particularly designed to parse files of newline-separated things,
like these git integers:

```
0
1
2
3
4
```

Load your ints with ease:

```rust,ignore
// Create the file of test data
use std::fs;
let tmp_dir = TempDir::new("tmp").unwrap();
let file_path = tmp_dir.path().join("list");
fs::write(&file_path, "0\n1\n2\n3\n4").unwrap();

// Load from file. Note that each element could result in an individual
// I/O or parse error. Here those are converted into a single `Result<Vec<u32>, _>`.
let v = from_file_lines(&file_path);
let v: Vec<Result<u32, _>> = v.unwrap().collect();
let v: Result<Vec<u32>, _> = v.into_iter().collect();
let v = v.unwrap();
assert!(v == vec![0, 1, 2, 3, 4]);
```

[Documentation](https://docs.rs/pars_list).

## License

This work is distributed under the super-Rust quad-license:

[Apache-2.0]/[MIT]/[BSL-1.0]/[CC0-1.0]

This is equivalent to public domain in jurisdictions that allow it (CC0-1.0).
Otherwise it is compatible with the Rust license, plus the option of the
runtime-exception-containing BSL-1. This means that, outside of public domain
jurisdictions, the source must be distributed along with author attribution and
at least one of the licenses; but in binary form no attribution or license
distribution is required.

[Apache-2.0]: https://opensource.org/licenses/Apache-2.0
[MIT]: https://www.opensource.org/licenses/MIT
[BSL-1.0]: https://opensource.org/licenses/BSL-1.0
[CC0-1.0]: https://creativecommons.org/publicdomain/zero/1.0
