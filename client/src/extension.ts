import * as path from "path";
import { ExtensionContext } from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions
} from "vscode-languageclient/node";

let client: LanguageClient;

function serverTarget(): string {
    const arch = process.arch === "x64" ? "x64" : process.arch === "arm64" ? "arm64" : process.arch;

    if (process.platform === "darwin") {
        return `darwin-${arch}`;
    }

    if (process.platform === "linux") {
        return `linux-${arch}`;
    }

    if (process.platform === "win32") {
        return `win32-${arch}`;
    }

    return `${process.platform}-${arch}`;
}

export function activate(ctx: ExtensionContext) { 
    const serverPath = ctx.asAbsolutePath(
        path.join(serverTarget(), "server", "bin", process.platform === "win32" ? "snek-lsp.exe" : "snek-lsp")
    );

    const serverOptions: ServerOptions = {
        command: serverPath,
        args: []
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "snek-lang" }]
    };

    client = new LanguageClient(
        "snek-lsp",
        "Snek Language Server",
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate() {
    return client?.stop();
}
