<h1 align="center">Chronolog (Scheduling Library)</h1>
<img src="assets/Chronolog Banner.png" />

Chronolog is the **ULTIMATE** unopinionated scheduling library which all developers dreamed of.
Schedule thousands of tasks with the efficiency of rust while still using it in multiple programming languages
such as <u>Python</u>, <u>Rust</u>, <u>Javascript(and Typescript)</u> and even <u>Java</u>. The library is designed to be
as easy to use as possible while being powerful, flexible and extendable

<img align="center" src="assets/Chronolog Divider.png" />
Since Chronolog is a fully featured scheduling library, it provides many features out of the box by default:

- **📅 Schedules:** Attach tasks to specific times, time intervals, or even dynamically computed points 
in the future. Perfect for daily jobs, periodic checks, or event-based triggers.
<br /> <br />
- **🔄 Task Behavior And Management:** Fine grain control over the behavior of the task, such as when it overlaps, in addition
providing management of the task at runtime, such as adding, removing and rescheduling a task
<br /> <br />
- **📋 Task Composition:** Compose tasks in multiple ways, no restrictions on one and only behavior, you can go
as simple as retrying a task n times to even more complex operations such as parallel execution of tasks at a specific
time with timeouts
<br /> <br />
- **📡 Cross Language Communication** Write task logic in rust and handle the task in python. Finally, no more
FFI or gRPC shenanigans to communicate between programming language processes. Chronolog is the central hub for scheduling

<img align="center" src="assets/Chronolog Divider.png" />
Why use chronolog when other scheduling libraries exist in other programming languages? Some of the highlights which
you might consider to use chronolog over other scheduling libraries are:

- **🌐 Multi-language Support:** Chronolog is available in Python, Rust, JavaScript/TypeScript, and Java. 
Switch between languages without rewriting scheduling logic. No more trying to combat the limitations of different 
schedulers
<br /> <br />
- **🛠️ Extensible:** Chronolog's architecture has extensibility in mind, as such you are not restricted to using the
default implementation of the scheduler, tasks and even schedules. You can build extensions for chronolog in your favourite
programming language ecosystem
<br /> <br />
- **🚀 Lightweight & Efficient:** Minimal overhead ensures it won’t bloat your project, while still providing reliable 
timing and execution for thousands of tasks with the power, flexibility and safety of <u>_Rust_</u> under the hood as
its core.
<br /> <br />
- **🔧 Developer-Friendly:** Clear API, intuitive task registration, vast documentation, you name it. No complications, 
no trickery, what you write in code is what you will get in the production environment
<br /><br />
- **📦 Tiny But Mighty** Tired of large sized packages, taking forever to compile, consuming disk space and so on? We too,
as such, Chronolog is tiny about **~1MB** in size
<img align="center" src="assets/Chronolog Divider.png" />
When it comes to contributing and forking. Chronolog is free and open source to use, only restricted by the lightweight
**Apache License v2.0**. Contributions are welcome with wide open arms, chronolog is looking to foster a community