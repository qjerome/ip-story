# ip-story

A service used to store various IP address related information

# Testing the project

> ⚠️ `npm` (to build the frontend) must be available on the machine building the project

1. Run the project

```bash
# one must configure the URL of the backend storage
REDIS_URL=redis://localhost:6666 cargo run --bin ip-story
```

2. Visit http://localhost:8000
