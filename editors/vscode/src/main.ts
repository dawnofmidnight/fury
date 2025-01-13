import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";

const enum FuryCommands {
    RestartServer = "fury.restartServer",
}

let client: lc.LanguageClient | undefined;
let configureLang: vscode.Disposable | undefined;

export async function activate(context: vscode.ExtensionContext) {
    configureLang = vscode.languages.setLanguageConfiguration("fury", {
        onEnterRules: [...continueTypingCommentsOnNewline()],
    });

    const restartCommand = vscode.commands.registerCommand(
        FuryCommands.RestartServer,
        async () => {
            if (!client) {
                vscode.window.showErrorMessage("fury client not found");
                return;
            }
            try {
                if (client.isRunning()) {
                    await client.restart();
                    vscode.window.showInformationMessage("fury server restarted.");
                } else {
                    await client.start();
                }
            } catch (err) {
                client.error("Restarting client failed", err, "force");
            }
        },
    );

    context.subscriptions.push(restartCommand);

    client = await createLanguageClient();
    client?.start();
}

export function deactivate(): Thenable<void> | undefined {
    configureLang?.dispose();

    return client?.stop();
}

function continueTypingCommentsOnNewline(): vscode.OnEnterRule[] {
    const indentAction = vscode.IndentAction.None;

    return [
        {
            beforeText: /^\s*\/{2}!.*$/,
            action: { indentAction, appendText: "//! " },
        },
        {
            beforeText: /^\s*\/{3}.*$/,
            action: { indentAction, appendText: "/// " },
        },
        {
            beforeText: /^\s*\/{2}.*$/,
            action: { indentAction, appendText: "// " },
        },
    ];
}

async function createLanguageClient(): Promise<lc.LanguageClient | undefined> {
    const command = await getFuryCommandPath();
    if (!command) {
        const message = `Could not resolve Fury executable. Please ensure it is available
    on the PATH used by VSCode or set an explicit "fury.path" setting to a valid Fury executable.`;

        vscode.window.showErrorMessage(message);
        return;
    }

    const clientOptions: lc.LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "fury" }],
        synchronize: {
            fileEvents: [
                vscode.workspace.createFileSystemWatcher("**/fury.toml"),
            ],
        },
    };

    const serverOptions: lc.ServerOptions = {
        command,
        args: ["lsp"],
        options: { env: process.env, },
    };

    return new lc.LanguageClient(
        "fury",
        "Fury",
        serverOptions,
        clientOptions,
    );
}

export async function getFuryCommandPath(): Promise<string | undefined> {
    const command = getWorkspaceConfigFuryExePath();
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!command || !workspaceFolders) {
        return command ?? "fury";
    } else if (!path.isAbsolute(command)) {
        for (const workspace of workspaceFolders) {
            const commandPath = path.resolve(workspace.uri.fsPath, command);
            if (await fileExists(commandPath)) {
                return commandPath;
            }
        }
        return undefined;
    }
    return command;
}

const EXTENSION_NS = "fury";

function getWorkspaceConfigFuryExePath(): string | undefined {
    const exePath = vscode.workspace.getConfiguration(EXTENSION_NS).get("path");
    return typeof exePath !== "string" || !exePath || exePath.trim().length === 0
        ? undefined
        : exePath;
}

function fileExists(executableFilePath: string): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
        fs.stat(executableFilePath, (err, stat) => {
            resolve(err == null && stat.isFile());
        });
    }).catch(() => {
        // ignore all errors
        return false;
    });
}
