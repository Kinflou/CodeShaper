export function named(argument: unknown): string | undefined {
    return typeof argument === "string" ? argument : undefined;
}
