<h1 align="center">Chronolog (Scheduling Library)</h1>
<img src="assets/Chronolog Banner.png" />

> [!IMPORTANT]  
> The project is in its infancy, it is not out there, as its being worked on and crafted meticulously. If you plan to
> contribute to the project, now is the time to provide a good helping hand for the hardworking team. When it comes to
integrating with other programming languages, we mainly focus on rust then slowly make the features availaible in other 
languages

Chronolog is the **ULTIMATE** unopinionated scheduling library which all developers dreamed of.
Schedule thousands of tasks with the efficiency of rust while still using it in multiple programming languages
such as <u>Python</u>, <u>Rust</u>, <u>Javascript(and Typescript)</u> and even <u>Java</u>. The library is designed to be
as easy to use as possible while being powerful, flexible and extendable

<img align="center" src="assets/Chronolog Divider.png" />
Since Chronolog is a fully featured scheduling library, it provides many features out of the box by default:

## 🧩 Task Composition
Instead of thinking a task is just some executable, chronolog thinks of tasks as components in a group instead of 
single execution blocks, allowing the expression and reuse of complex logic easily, tasks consist of:
  - ***Task Metadata:*** The <ins>State</ins> of the task, anything data-related. Internally, this data is mutable, metadata
  can also be exposed restricting write access to any data, think of it as a global state for the entire task. In most
  cases this is optional to provide
  <br /> <br />
  - ***Task Frame*** The Task Frame is the core embodiment of a task. It defines <ins>What</ins> needs to be done. Think of it 
  as the immutable recipe or the instruction set for a specific unit of work. Task frames can access the metadata of the
  task
  <br /> <br />
  - **Task Schedule** This defines <ins>When</ins> the task needs to be executed, schedules can be as simple as an
  interval, to a cron expression and even a calendar. Given a specific time, they calculate based on the time provided, when
  the task will execute again (they can fail depending on the scenario)
  
## 🔄 Task Behavior And Management
Fine grain control over the behavior of the task, such as how it works when it overlaps, listening to specific task frame events. 
In addition, Chronolog provides management of the task at runtime, such as adding, removing and rescheduling a task

## 📋 Task Frame Decoration 
Compose task frames in multiple ways, no restrictions on one and only behavior, you can go
as simple as retrying a task n times to even more complex operations such as parallel execution of task frames
with timeout. True flexibility in how you write and maintain your task logic
## 📡 Cross Language Communication
Write the logic of a task in rust, schedule it in python, alert javascript listeners. Finally, no more FFI or gRPC 
shenanigans to communicate between programming language processes. Chronolog is the central hub for scheduling, invoke
from anywhere, schedule from anywhere, you name it!

<img align="center" src="assets/Chronolog Divider.png" />
Why use chronolog when other scheduling libraries exist in other programming languages? Some of the highlights / strength points
which you might consider to use chronolog over other scheduling libraries are:

- **🌐 Multi-language Support:** Chronolog is available in Python, Rust, JavaScript/TypeScript, and Java. 
Switch between languages without rewriting scheduling logic and learning a new framework. No more trying to combat the limitations of different 
schedulers
<br /> <br />
- **🛠️ Extensible:** Chronolog's architecture has extensibility in mind, as such you are not restricted to using the
default implementation of the scheduler, task frames, and even schedules. You can build extensions 
for chronolog in your favourite programming language ecosystem
<br /> <br />
- **🚀 Lightweight & Efficient:** Minimal overhead ensures it won’t bloat your project, while still providing reliable 
timing and execution for thousands of tasks with the power, flexibility and safety of <u>_Rust_</u> under the hood as
its core.
<br /> <br />
- **🔧 Developer-Friendly:** Clear API, intuitive task registration, vast documentation, life shouldn't be harder than
it needs to be. No complications, no trickery, what you write in code is what you will get in the production environment
<br /><br />
- **⏰ Millisecond Precision:** Chronolog is also designed to be millisecond precise, which makes it very practical for
frequent scheduled tasks, it maintains this precision even when clogged by multiple tasks
<br /> <br />
- **📦 Tiny But Mighty** Tired of large sized packages, taking forever to compile, consuming disk space and so on? We too,
as such, Chronolog is tiny about **~1MB** in size
<img align="center" src="assets/Chronolog Divider.png" />
When it comes to contributing and forking. Chronolog is free and open source to use, only restricted by the lightweight
**Apache License v2.0**. Contributions are welcome with wide open arms, chronolog is looking to foster a community