import argparse

import balatro_engine as engine
from simulation.training_evaluation import evaluate_model, load_policy


def main():
    parser = argparse.ArgumentParser(
        description="Evaluate a schema-v1 full-game policy"
    )
    parser.add_argument("checkpoint")
    parser.add_argument("--episodes", type=int, default=10)
    parser.add_argument("--seed", type=int, default=1_000_000)
    parser.add_argument("--horizon", type=int, default=4096)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--decks", default="Red,Magic,Painted")
    parser.add_argument("--stake", default="White")
    args = parser.parse_args()
    model = load_policy(args.checkpoint, args.device)
    decks = tuple(getattr(engine.Deck, name.strip()) for name in args.decks.split(","))
    print(
        evaluate_model(
            model,
            args.episodes,
            args.seed,
            args.horizon,
            decks=decks,
            batch_size=args.batch_size,
            stake=getattr(engine.Stake, args.stake),
        )
    )


if __name__ == "__main__":
    main()
