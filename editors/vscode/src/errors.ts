import { MissingVariables } from "./variables/protocol";

export function missingVariables(error: unknown): MissingVariables | undefined {
    const data = error instanceof Error && "data" in error ? error.data : undefined;

    return data && typeof data === "object" && "project" in data && "variables" in data ? data as MissingVariables : undefined;
}

export function reason(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}
