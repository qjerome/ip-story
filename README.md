# ip-story
A service used to store various IP address related information

# Testing the project

1. Run the project
```bash
# one must configure the URL of the backend storage
REDIS_URL=redis://localhost:6666 cargo run
```
2. Extract API documentation
```bash
# by default the project will listen on http://127.0.0.1:8000
# get and unpack openapi documentation
curl http://localhost:8000/api/openapi/json | jq '.data' > openapi.json
```
3. Load OpenAPI documentation in a UI (https://redocly.github.io/redoc/)

