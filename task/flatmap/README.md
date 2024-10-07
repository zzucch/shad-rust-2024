В данной задаче вам предлагается реализовать flat map - структуру данных, которая хранит
пары ключ-значение в отсортированном векторе.

## Реализация

Реализуйте методы структуры FlatMap. Заметьте, что методы, осуществляющие поиск по ключу
(`get`, `remove`, `remove_entry`) должны работать не только с типом ключа, но с любым
типом, т.ч. `K: Borrow<B>` и `B: Ord + ?Sized` ([подробнее про Borrow](https://doc.rust-lang.org/std/borrow/trait.Borrow.html)). Подсказка: сигнатуры этих функций
в точности совпадают с сигнатурами одноимённых функций в `HashMap`.

Чтобы не писать бинарный поиск руками, используйте метод [binary_search_by](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search_by). Пример использования:

```rust
self.0.binary_search_by(|pair| pair.0.cmp(key))
```

Он возвращает `Ok(usize)`, если искомый элемент найден, и `Err(usize)`, если нет.
Во втором случае usize указывает на позицию, на которой элемент следует вставить,
чтобы сохранить порядок.

Также реализуйте для FlatMap следующие traits:
* [Index](https://doc.rust-lang.org/std/ops/trait.Index.html), чтобы можно было
использовать квадратные скобки для поиска ключа.
* [Extend](https://doc.rust-lang.org/std/iter/trait.Extend.html), чтобы можно было
расширить FlatMap элементами из итератора.
* `From<Vec(K, V)>` - если в векторе встречаются дубликаты по ключам, в FlatMap
должен попасть последний ключ. Используйте [sort_by](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by) и [dedup_by](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.dedup_by).
* `FromIterator<(K, V)>`
* `IntoIterator` - переиспользуйте итератор `Vec`, чтобы не писать свой итератор.

Также реализуйте `From<FlatMap<K, V>>` для `Vec<(K, V)>`.

## Бенчмарк

Можете побенчмаркать производительность своего решения относительно других стандартных
контейнеров командой `cargo bench`. Бенчмаркается только поиск ключа, т.к. на вставке
и удалении FlatMap очень плох, конечно.

Авторское решение показывает такие результаты:

```
100k_random_lookup_hits/flat_map
                        time:   [5.5829 ms 5.5865 ms 5.5908 ms]
100k_random_lookup_hits/btree_map
                        time:   [6.4700 ms 6.4747 ms 6.4801 ms]
100k_random_lookup_hits/hash_map
                        time:   [1.0479 ms 1.0507 ms 1.0536 ms]
100k_random_lookup_misses/flat_map
                        time:   [4.6493 ms 4.6605 ms 4.6766 ms]
100k_random_lookup_misses/btree_map
                        time:   [7.2692 ms 7.2754 ms 7.2826 ms]
100k_random_lookup_misses/hash_map
                        time:   [1.9376 ms 1.9405 ms 1.9440 ms]
```
