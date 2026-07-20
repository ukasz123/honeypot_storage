# Local LLM journey

### The goal
I wanted to create a service that could store (almost) anything that comes from the outside, making it possible to inspect it later. It was meant to be a place where I could send random things from environments I may not fully control to inspect later. Also, I wanted to inspect what random traffic from the web looks like.

### Process
I decided to use this idea as an experimental project where I could use a local LLM as a backend for an agent building the app. I wanted to check whether I am able to build something useful with just a not-so-powerful AI. I decided to avoid any manual coding, even if prompting could take more time and effort. On the other hand, I was reading all output from the agent to ensure there were no solutions I disagreed with, and (sometimes) I read third-party documentation (https://docs.rs) to give the agent useful hints and proper directions.
Every conversation thread was saved in [thread_dump](/thread_dump) folder.

### Setup
#### Hardware:
  - Macbook Pro 2021, Apple M1 Pro, 32 GB RAM as a server
  - Macbook Pro 2019, Intel Core i7, 16GB RAM as a client 
#### Software:
  - Backend:
    - LLM Studio running a single model at the time
    - `caffeinate` to stop laptop from going to sleep
  - Client:
    - Zed as an editor with configured LLM Studio as LLM provider for agent

## Initial commit to version 1.0 ([conversations](/thread_dump/google/gemma-4-26b-a4b))
I used Google's Gemma4 4bit([gemma-4-26b-a4b](https://lmstudio.ai/models/google/gemma-4-2026-07-12-a4b)) with a 136,960 context length, taking up ~27 GB of RAM (from ~27 GB of shared VRAM).
The project scaffolding and initial implementation were created quite quickly and without problems. Problems started when conversations were becoming longer and more code was created. Eventually it started to be a challenge to add new features or perform refactoring. Actually, refactoring could cause some extra random changes that would break functionality sometimes. Sometimes the agent went in circles, constantly doing one thing and reverting it without any progress — at that point the only solution was to break the thread and start a new session. In a few cases, I had to step in and make some changes manually or give precise instructions to the agent; otherwise, it could not find the right solution.
When given an impossible task (my mistake), it never said that it was wrong and [kept trying until stopped](thread_dump/google/gemma-4-26b-a4b/08_Refactor%20Rust%20save_request%20avoiding%20clones.md).
In general, code generation and thinking were rather slow compared to remote models. I could take some breaks even for rather small changes. Also, the model hallucinated a bit. It tended to come back to previous solutions even when they were rejected.
I noticed that when running LLM backend laptop took additional 35-40W of electricity power.

## DuckDB support
### GLM 4.7 Flash ([conversations](/thread_dump/z.ai/glm-4.7-flash-mix))
I decided to use different model and tested [glm-4.7-flash-mix](https://lmstudio.ai/models/zai-org/glm-4.7-flash) for planning but it was completely useless. It quickly started generating the same output again and again.
### Back to Gemma-4
When asked agent was able to prepare plan for adding support for DuckDB as alternative for SQLite. Process was split into two phases: refactoring of existing code - extracting trait for interacting with database and then adding feature and implementation backed by DuckDB. 
Implementation of first phase went fairly well but some compilation errors were present. They were fixed in separate session to avoid context pollution.
DuckDB support was added but implementation contained even more compilation errors and even tryin new sessions with prompt to fix compilation errors did not help and I eventually gave up on and decided to fix remaining issues by hand.

## Raspberry Pi support
Again I tried to use different models to add configuration and script that would build the service for Raspberry Pi 
### GLM 4.7 Flash ([conversation](thread_dump/z.ai/glm-4.7-flash-mix/02_Cross%20Compiling%20My%Drive%20for%20Raspberry%20Pi.md))
It quickly got stucked on repeating `<|observation|>` again and again while generating tokens.
### Gemma 4 - 4b variant ([conversation](thread_dump/google/gemma-4-4b/01_Cross%20compile%20my%20drive%20project.md))
After promising start LLM refused to use any provided tools to gather additional data and finally it requested me to provide it the solution.
### Gemma 4 - 26b ([conversation](thread_dump/google/gemma-4-26b-a4b/17_Rust%20project%20cross%20compile%20setup.md))
Finally this model was able to do the right job without any assistance and made no mistakes.

## Capturing `query` from requests 
- [Initial attemp](thread_dump/google/gemma-4-26b-a4b/18_Adding%20Query%20Column%20to%20Requests%20Table.md) - Agent failed quickly when agent was started in "Write" mode; it tried to read too much and eventually failed to prepare and execute any reasonable plan
- ["Ask" mode for plan](thread_dump/google/gemma-4-26b-a4b/19_Implementing%20Request%20Query%20Column%20Migrations.md) - It created plan with some incorrect steps but the first one was formed correctly. When asked to implement the first step (in "Write" mode) it ignored the fact it was supposed to support migration. After pointing out this issue it was able to correct itself and prepare migration. It even rejected (invalid) request to make the `query` collumn nullable. 👏 Unfortunately, it was unable to move on to the next part of the plan.
- [Update "INSERT" statements](thread_dump/google/gemma-4-26b-a4b/20_Store%20URI%20query%20in%20database.md) - In "Write" mode I asked specifically what agent was supposed to do. It did necessary modifications in one go without any problems.
- **AFTERMATCH** - when I tried to run the code after migration I realized that SQLite requires special threatment (running "ALTER TABLE" statements conditionally) so after digging myself I decided to use `migrate` feature from `sqlx` crate to update the database correctly. Luckily there was no issues with DuckDB database update.
