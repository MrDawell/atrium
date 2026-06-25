import argparse
import json
import os
import sys

# Ensure api directory is in import search path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Handle automatic compile on the fly if needed
try:
    import daemon_pb2
    import daemon_pb2_grpc
except ImportError:
    proto_dir = os.path.dirname(os.path.abspath(__file__))
    proto_file = os.path.join(proto_dir, "daemon.proto")
    if os.path.exists(proto_file):
        try:
            from grpc_tools import protoc
            protoc.main((
                '',
                f'-I{proto_dir}',
                f'--python_out={proto_dir}',
                f'--grpc_python_out={proto_dir}',
                proto_file,
            ))
            import daemon_pb2
            import daemon_pb2_grpc
        except ImportError:
            print("Error: Could not import 'daemon_pb2' and 'grpc_tools' is not installed.", file=sys.stderr)
            print("Please run: pip install grpcio grpcio-tools", file=sys.stderr)
            sys.exit(1)
    else:
        print("Error: generated proto modules not found and daemon.proto is missing.", file=sys.stderr)
        sys.exit(1)

import grpc

def get_grpc_stub():
    channel = grpc.insecure_channel('127.0.0.1:50051')
    return daemon_pb2_grpc.DaemonServiceStub(channel)

def run_cli_index(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.IndexRequest(repo_path=os.path.abspath(args.path))
        resp = stub.IndexRepository(req)
        print(f"Indexing completed: success={resp.success}")
        print(f"Indexed Symbols count: {resp.indexed_symbols}")
        print(f"Server Message: {resp.message}")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_context(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.ContextRequest(
            task_description=args.task,
            focus_files=args.files or []
        )
        resp = stub.GetContextForTask(req)
        print(f"Reasoning:\n{resp.reasoning}\n")
        print("Relevant Files Outlines:")
        for f in resp.files:
            print(f"--- File: {f.path} (Relevance: {f.relevance_score}) ---")
            print(f.snippet)
            print("-" * 40)
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_verify(args):
    stub = get_grpc_stub()
    try:
        if os.path.exists(args.patch):
            with open(args.patch, 'r', encoding='utf-8') as f:
                patch_content = f.read()
        else:
            patch_content = args.patch

        req = daemon_pb2.VerifyRequest(
            patch_content=patch_content,
            target_branch=args.branch
        )
        resp = stub.VerifyPatch(req)
        print(f"Verification is_valid: {resp.is_valid}")
        print("\nCompile Errors:")
        for err in resp.compile_errors:
            print(err)
        print(f"\nLinter Output:\n{resp.linter_output}")
        print(f"\nTest Output:\n{resp.test_output}")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def execute_mcp_tool(stub, tool_name, arguments):
    try:
        if tool_name == "index_repository":
            repo_path = arguments.get("repo_path")
            if not repo_path:
                return {"isError": True, "content": [{"type": "text", "text": "repo_path is required"}]}
            req = daemon_pb2.IndexRequest(repo_path=os.path.abspath(repo_path))
            resp = stub.IndexRepository(req)
            return {
                "content": [
                    {
                        "type": "text",
                        "text": f"Index status: success={resp.success}, indexed_symbols={resp.indexed_symbols}\nMessage: {resp.message}"
                    }
                ]
            }
            
        elif tool_name == "get_context":
            task_desc = arguments.get("task_description", "")
            focus_files = arguments.get("focus_files", [])
            req = daemon_pb2.ContextRequest(task_description=task_desc, focus_files=focus_files)
            resp = stub.GetContextForTask(req)
            
            files_info = []
            for f in resp.files:
                files_info.append(f"File: {f.path}\nRelevance: {f.relevance_score}\nSnippet Outline:\n{f.snippet}\n")
            
            output_text = f"Reasoning:\n{resp.reasoning}\n\n" + "\n".join(files_info)
            return {
                "content": [{"type": "text", "text": output_text}]
            }
            
        elif tool_name == "verify_patch":
            patch_content = arguments.get("patch_content")
            target_branch = arguments.get("target_branch", "main")
            if not patch_content:
                return {"isError": True, "content": [{"type": "text", "text": "patch_content is required"}]}
            req = daemon_pb2.VerifyRequest(patch_content=patch_content, target_branch=target_branch)
            resp = stub.VerifyPatch(req)
            
            output_text = f"Is Valid: {resp.is_valid}\n\nCompile Errors:\n" + "\n".join(resp.compile_errors)
            output_text += f"\n\nLinter Output:\n{resp.linter_output}\n\nTest Output:\n{resp.test_output}"
            return {
                "content": [{"type": "text", "text": output_text}]
            }
            
        else:
            return {"isError": True, "content": [{"type": "text", "text": f"Unknown tool: {tool_name}"}]}
    except Exception as e:
        return {"isError": True, "content": [{"type": "text", "text": f"gRPC server call failed: {str(e)}"}]}

def run_mcp_server():
    stub = get_grpc_stub()
    
    # Read/Write lines in stdio loop complying with MCP specification
    for line in sys.stdin:
        if not line:
            break
        try:
            req = json.loads(line)
            method = req.get("method")
            req_id = req.get("id")
            
            if method == "initialize":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "atrium-mcp",
                            "version": "1.0.0"
                        }
                    }
                }
            elif method == "tools/list":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": [
                            {
                                "name": "index_repository",
                                "description": "Index a repository's source code files for Atrium.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "repo_path": {"type": "string", "description": "Absolute path to the repository"}
                                    },
                                    "required": ["repo_path"]
                                }
                            },
                            {
                                "name": "get_context",
                                "description": "Retrieve high-signal symbol structures and durable facts for a list of focused files.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "task_description": {"type": "string", "description": "The coding task description"},
                                        "focus_files": {
                                            "type": "array",
                                            "items": {"type": "string"},
                                            "description": "List of files to retrieve context for"
                                        }
                                    },
                                    "required": ["task_description", "focus_files"]
                                }
                            },
                            {
                                "name": "verify_patch",
                                "description": "Execute local verification checks (cargo check, cargo test, clippy) against a proposed patch.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "patch_content": {"type": "string", "description": "Unified diff patch content"},
                                        "target_branch": {"type": "string", "description": "Target branch to test against"}
                                    },
                                    "required": ["patch_content", "target_branch"]
                                }
                            }
                        ]
                    }
                }
            elif method == "tools/call":
                params = req.get("params", {})
                tool_name = params.get("name")
                arguments = params.get("arguments", {})
                
                result = execute_mcp_tool(stub, tool_name, arguments)
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": result
                }
            else:
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                }
                
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()
        except Exception as e:
            sys.stderr.write(f"MCP Exception: {str(e)}\n")
            sys.stderr.flush()

def main():
    parser = argparse.ArgumentParser(description="Atrium Bridge Client / MCP Server")
    subparsers = parser.add_subparsers(dest="command")

    # Index Subcommand
    p_index = subparsers.add_parser("index", help="Index a repository path")
    p_index.add_argument("--path", required=True, help="Path to repository")

    # Context Subcommand
    p_context = subparsers.add_parser("context", help="Request task context")
    p_context.add_argument("--task", required=True, help="Task description")
    p_context.add_argument("--files", nargs="+", help="Focused file paths")

    # Verify Subcommand
    p_verify = subparsers.add_parser("verify", help="Verify proposed code patch")
    p_verify.add_argument("--patch", required=True, help="Patch text or path to diff file")
    p_verify.add_argument("--branch", default="main", help="Target branch name")

    # MCP Server Subcommand
    subparsers.add_parser("mcp", help="Launch bridge in Model Context Protocol (MCP) server mode")

    args = parser.parse_args()

    if args.command == "index":
        run_cli_index(args)
    elif args.command == "context":
        run_cli_context(args)
    elif args.command == "verify":
        run_cli_verify(args)
    elif args.command == "mcp":
        run_mcp_server()
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
