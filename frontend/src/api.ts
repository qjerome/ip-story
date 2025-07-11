// Define a type for API methods
type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH";

// Define a type for API path parameters
interface PathParams {
  [key: string]: string | number;
}

// Define a type for API endpoint configuration
interface ApiEndpoint {
  method: HttpMethod;
  path: string;
}

// Define a type for API endpoint configuration
interface ApiEndpoint {
  method: HttpMethod;
  path: string;
}

// Define a type for API configuration
interface ApiConfig {
  baseUrl?: string | null;
  endpoints: {
    openApi: ApiEndpoint;
  };
}

// Example API configuration based on the provided OpenAPI description
export const api: ApiConfig = {
  baseUrl: null,
  endpoints: {
    openApi: {
      method: "GET",
      path: "/api/openapi/json",
    },
  },
};

export function apiUrl(
  endpoint: ApiEndpoint,
  pathParams?: PathParams,
  queryParams?: URLSearchParams
): string {
  let url =
    api.baseUrl == null ? `${endpoint.path}` : `${api.baseUrl}${endpoint.path}`;

  // Replace path parameters
  if (pathParams) {
    for (const key in pathParams) {
      if (!url.includes(`{${key}}`)) {
        console.error(`cannot replace path param ${key} in ${url}`);
        continue;
      }

      url = url.replace(
        `{${key}}`,
        encodeURIComponent(String(pathParams[key]))
      );
    }
  }

  // Append query parameters
  if (queryParams) {
    url += `?${queryParams.toString()}`;
  }

  return url;
}

export function apiRequest(
  endpoint: ApiEndpoint,
  pathParams?: PathParams,
  queryParams?: URLSearchParams,
  body?: BodyInit
): RequestInfo {
  return new Request(apiUrl(endpoint, pathParams, queryParams), {
    method: endpoint.method,
    body: body,
  });
}

function logApiError(
  message: string,
  input: RequestInfo | URL,
  init?: RequestInit
) {
  if (init) {
    console.error(`request=${input} init=${init} error=${message}`);
  } else {
    console.error(`request=${input} error=${message}`);
  }
}

export async function fetchAPI<T>(
  input: RequestInfo | URL,
  init?: RequestInit
): Promise<T | null> {
  try {
    const response = await fetch(input, init);
    if (response.ok) {
      if (response.headers.get("content-type") != "application/json") {
        logApiError("api endpoint didn't return json content", input, init);
        return null;
      }

      const json = await response.json();
      if (json["error"]) {
        logApiError(`api endpoint error: ${json["error"]}`, input, init);
        return null;
      }
      if (json["data"]) {
        const data: T = json["data"];
        return data;
      }
    } else {
      logApiError(
        `unexpected status from search API: ${response.status}`,
        input,
        init
      );
    }
  } catch (error) {
    logApiError(`caught exception while querying API: ${error}`, input, init);
  }

  return null;
}
