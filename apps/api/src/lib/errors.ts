/**
 * EXTERNAL_API_CONTRACT §5 错误模型：统一 envelope
 * `{ "error": { "code", "message", "details", "request_id", "retryable" } }`，
 * 12 类封闭词表，每个错误必须明确 `retryable`。
 */
import { randomUUID } from "node:crypto";

export type ErrorCode =
  | "AUTHENTICATION_FAILED"
  | "AUTHORIZATION_DENIED"
  | "VALIDATION_ERROR"
  | "RESOURCE_NOT_FOUND"
  | "RESOURCE_CONFLICT"
  | "RESOURCE_UNAVAILABLE"
  | "CAPABILITY_UNSUPPORTED"
  | "COMMAND_REJECTED"
  | "COMMAND_FAILED"
  | "DEPENDENCY_UNAVAILABLE"
  | "RATE_LIMITED"
  | "INTERNAL_ERROR";

export interface ErrorEnvelope {
  error: {
    code: ErrorCode;
    message: string;
    details?: Record<string, unknown>;
    request_id: string;
    retryable: boolean;
  };
}

/** 携带 taxonomy 语义的应用错误；route/handler 抛出后由统一错误处理器出 envelope。 */
export class ApiError extends Error {
  readonly code: ErrorCode;
  readonly status: number;
  readonly retryable: boolean;
  readonly details: Record<string, unknown> | undefined;

  constructor(
    code: ErrorCode,
    status: number,
    message: string,
    retryable: boolean,
    details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
    this.retryable = retryable;
    this.details = details;
  }
}

export function errorEnvelope(err: ApiError): ErrorEnvelope {
  const body: ErrorEnvelope = {
    error: {
      code: err.code,
      message: err.message,
      request_id: randomUUID(),
      retryable: err.retryable,
    },
  };
  if (err.details !== undefined) body.error.details = err.details;
  return body;
}

export function validationError(message: string): ApiError {
  return new ApiError("VALIDATION_ERROR", 400, message, false);
}

export function notFound(message: string): ApiError {
  return new ApiError("RESOURCE_NOT_FOUND", 404, message, false);
}

/** §6 默认安全模型：认证失败 401（retryable=false）。message 保持通用，不回显 credential 语境细节。 */
export function authenticationFailed(message = "authentication required"): ApiError {
  return new ApiError("AUTHENTICATION_FAILED", 401, message, false);
}

/** §6 默认安全模型：已认证但未授权 403（fail-closed，retryable=false）。 */
export function authorizationDenied(message = "principal is not authorized for this operation"): ApiError {
  return new ApiError("AUTHORIZATION_DENIED", 403, message, false);
}

/** §5 RATE_LIMITED：retryable=true；调用方需同时设置 Retry-After 响应头。 */
export function rateLimited(message = "rate limit exceeded for this principal and action"): ApiError {
  return new ApiError("RATE_LIMITED", 429, message, true);
}

export function dependencyUnavailable(message: string): ApiError {
  return new ApiError("DEPENDENCY_UNAVAILABLE", 503, message, true);
}

export function internalError(message: string): ApiError {
  return new ApiError("INTERNAL_ERROR", 500, message, false);
}
