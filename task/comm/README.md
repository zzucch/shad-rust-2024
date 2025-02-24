В данной задаче вам предлагается написать утилиту командной строки, которая принимает на вход два файла и выводит в stdout все строки, которые встречаются в обoих файлах. Заметьте, что каждая уникальная строчка из пересечения файлов должна быть выведена ровно один раз. Порядок вывода не специфицируется.

## Реализация

* Чтобы прочитать аргументы командной строки в вектор, используйте `std::env::args`:

```rust
let args = std::env::args().collect::<Vec<String>>();
```

* Построчное чтение файла осуществляется так:

```rust
use std::io::BufRead;

let file = std::fs::File::open(path).unwrap();
let reader = std::io::BufReader::new(file);
for line in reader.lines() {
    // ...
}
```

* Для эффективного построения пересечения двух файлов используйте [HashSet](https://doc.rust-lang.org/stable/std/collections/struct.HashSet.html). Вам понадобятся методы `insert` и `contains` (или `take`).

* Запись в stdout в простом виде осуществляется вызовом `println!("{}", line)`.

* Чтобы не писать много раз `std::`, можно вставить в начало файла что-то такое:

```rust
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufRead, BufReader},
};
```

## Запуск

Чтобы позапускать своё приложение руками, используйте команду:

```
cargo run --release -- file1 file2
```

Также можете сделать `cargo build --release`, бинарник будет в `target/release/comm`.

## Тестирование

Для запуска тестов используйте команду:

```
cargo xtask check
```

В данной задаче тесты интеграционные - ваш код компилируется в самостоятельный бинарник и запускается как чёрный ящик.
Это значит, что использовать дебагер в тестах не получился, но пофейлившиеся тесты дампят входные данные в stderr, так что можно подебажить плохой вход руками.

## Бенчмарк

Тесты проверяют лишь корректность кода, а не его скорость, но из любопытства можете сравнить производительность своего кода с референсной реализацией такой же программы на С++ (находится в `src/main.cpp`).

Запуск бенчмарка осуществляется командой:

```
cargo xtask bench
```

Для успешного запуска вам понадобится компилятор C++ (консольная команда `c++`).

Авторское решение показывает такие результаты:

```
50k_50k/rust            time:   [30.752 ms 30.791 ms 30.859 ms]
50k_50k/cpp             time:   [70.117 ms 71.271 ms 72.469 ms]
0_100k/rust             time:   [28.652 ms 28.780 ms 28.950 ms]
0_100k/cpp              time:   [79.209 ms 84.098 ms 88.142 ms]
```
