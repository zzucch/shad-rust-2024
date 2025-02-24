В этой задаче вам предлагается реализовать многопоточный планировщик для асинхронного рантайма.

> Данная папка соответствует сразу двум задачам: `rio` и `rio-net`.
> К `rio` относится код под feature-флагом "rt-multi-thread". Код под флагом "net"
> не используется сборкой. Сборка и тестирование задачи `rio` устроены как обычно:
> `cargo xtask check` прогоняет тесты, `cargo xtask submit` отправляет решение.

## 1. Описание интерфейса

Вам необходимо реализовать интерфейс планировщика в файле `src/scheduler/multi_thread.rs`. Он состоит из двух функций:

* `new(context_manager, thread_count)` - создать новый многопоточный планировщик с указанным числом потоков. Каждая задача, исполняемая планировщиком, должна быть в контексте текущего рантайма - для этого используйте передаваемый в конструктор context manager:
	- Метод `install()` усталавливает контекст глобально для текущего потока
	- Метод `enter()` устанавливает контекст и возвращает guard, который очистит контекст в деструкторе
* `submit(task)` - создать асинхронную таску, исполняемую конкурентно с текущим потоком. Контракт исполнения следующий:
	1. Сразу после сабмита нужно позвать `task.poll(cx)` (но уже в другом треде)
	2. После каждого вызова `waker.wake()` должен происходить новый `task.poll(cx)` (waker передаётся в `poll` как часть контекста первым аргументом).
		- Если `wake` был позван несколько раз до момента вызова `poll`, то поллить таску больше одного раза не нужно.
		- Если `wake` был позван в тот момент, когда таска уже поллится другим потоком, то нужно запланировать новый `poll` после завершения текущего.

## 2. Реализация

* В соседнем файле реализован однопоточный планировщик. Почитайте его - многие элементы многопоточного планировщика очень похожи: `src/scheduler/current_thread.rs`.
* В качестве тред пула используйте `rayon::ThreadPool`:
	- `.spawn_fifo(op)` позволяет исполнить замыкание каким-то потоком из тред пула, соблюдая FIFO.
	- `.broadcast(op)` позволяет исполнить замыкание всеми потоками по очереди. Так вы можете установить контекст рантайма.
* Находясь в контексте тред пула, можно звать глобальную функцию `rayon::spawn_fifo`. Это позволяет не таскать ссылку на тред пул.

## 3. Отладка

* Заметьте: если замыкание, переданное в `spawn_fifo`, паникует, это приводит к аборту всего процесса. Чтобы увидеть нормальный трейсбек, запустите тест в отладчике. Или установите panic handler на тред пуле.
* Можете пользоваться логированием (макросы `log::{trace, debug, info, warn, error}`). Запустить конкретный тест и посмотреть логи можно следующей командой:
  - `RUST_LOG=debug cargo test test_name -- --nocapture`
* Для продвиутых есть `tracing`, но его настраивайте сами :)
