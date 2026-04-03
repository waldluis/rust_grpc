import argparse
import grpc

import backend_pb2
import backend_pb2_grpc


def setup(stub, procedure: str):
    request = backend_pb2.SetupRequest(procedure=procedure)
    response = stub.Setup(request)
    print_response(response)


def execute(stub):
    request = backend_pb2.ExecuteRequest()
    response = stub.Execute(request)
    print_response(response)


def terminate(stub):
    request = backend_pb2.TerminateRequest()
    response = stub.Terminate(request)
    print_response(response)


def print_response(response):
    print(f"Success: {response.success}")
    print(f"Message: {response.message}")


def main():
    parser = argparse.ArgumentParser(description="gRPC CLI client for Backend service")
    
    parser.add_argument(
        "--host",
        default="localhost",
        help="Backend host (default: localhost)"
    )
    parser.add_argument(
        "--port",
        default="50051",
        help="Backend port (default: 50051)"
    )

    subparsers = parser.add_subparsers(dest="command", required=True)

    # Setup command
    parser_setup = subparsers.add_parser("setup", help="Call Setup")
    parser_setup.add_argument(
        "--procedure",
        required=True,
        help="Procedure name"
    )

    # Execute command
    subparsers.add_parser("execute", help="Call Execute")

    # Terminate command
    subparsers.add_parser("terminate", help="Call Terminate")

    args = parser.parse_args()

    target = f"{args.host}:{args.port}"
    channel = grpc.insecure_channel(target)
    stub = backend_pb2_grpc.BackendStub(channel)

    if args.command == "setup":
        setup(stub, args.procedure)
    elif args.command == "execute":
        execute(stub)
    elif args.command == "terminate":
        terminate(stub)


if __name__ == "__main__":
    main()