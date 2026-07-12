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
