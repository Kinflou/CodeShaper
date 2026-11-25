import { ProjectVariables, Source, Variable } from "./protocol";

const LENGTH = 60;

const SOURCES: Record<Source, string> = {
    constant: "constant",
    set: "set here",
    configuration: "configuration",
    default: "default",
    missing: "not set"
};

export interface Row {
    name: string;
    description: string;
    tooltip: string;
    icon: string;
    context: "constant" | "variable" | "set" | "unknown";
}

export function rows({ variables, unknown }: ProjectVariables): Row[] {
    return [...variables.map(row), ...unknown.map(unknownRow)];
}

export function shown(value: string): string {
    const line = value.replace(/\r?\n/g, "⏎").replace(/\t/g, "⇥");
    const characters = [...line];

    return characters.length > LENGTH ? `${characters.slice(0, LENGTH).join("")}…` : line;
}

function row(variable: Variable): Row {
    const { name, value, source } = variable;

    return {
        name,
        description: description(variable),
        tooltip: value === null ? `${name} has no value` : `${name} = ${value}`,
        icon: source === "missing" ? "warning" : source === "constant" ? "symbol-constant" : "symbol-variable",
        context: source === "constant" || source === "set" ? source : "variable"
    };
}

function description({ value, source }: Variable): string {
    if (value === null) {
        return SOURCES[source];
    }

    return `${value === "" ? "empty" : shown(value)}  (${SOURCES[source]})`;
}

function unknownRow(name: string): Row {
    return {
        name,
        description: "set here, but the project has no such variable",
        tooltip: `${name} isn't used`,
        icon: "circle-slash",
        context: "unknown"
    };
}
