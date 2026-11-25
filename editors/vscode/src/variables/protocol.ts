export type Source = "constant" | "set" | "configuration" | "default" | "missing";

export interface Variable {
    name: string;
    kind: "constant" | "variable";
    value: string | null;
    source: Source;
    otherwise: string | null;
    range: { start: { line: number; character: number }; end: { line: number; character: number } } | null;
}

export interface ProjectVariables {
    name: string;
    variables: Variable[];
    unknown: string[];
}

export interface MissingVariables {
    project: string;
    variables: string[];
}
