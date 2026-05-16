import * as path from "path";
import { ExtensionContext } from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(ctx: ExtensionContext) { 
    const serverPath = ctx.asAbsolutePath(
        path.join("server", "target", "debug", process.platform === "win32" ? "snek-lsp.exe" : "snek-lsp")
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