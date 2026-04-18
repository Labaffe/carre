# Minimal Bevy Tweening

Simple ECS-based tweening system for Bevy.


## Basic usage

Tween X position:

```rust
commands.entity(entity).insert(
    TweenSequence::<TranslationX>::new(
        Tween::new(0.0, 300.0, 0.5, Ease::OutQuad)
    )
);
```

UI position (X and Y independently):

```rust
commands.entity(entity).insert((
    TweenSequence::<StyleLeft>::new(
        Tween::new(0.0, 200.0, 0.5, Ease::OutQuad)
    ),
    TweenSequence::<StyleTop>::new(
        Tween::new(0.0, 100.0, 0.5, Ease::OutQuad)
    ),
));
```

## Chaining

```rust
commands.entity(entity).insert(
    TweenSequence::<TranslationX>::new(
        Tween::new(0.0, 300.0, 0.5, Ease::OutQuad)
    )
    .then(Tween::new(300.0, 100.0, 0.3, Ease::InQuad))
);
```

## Custom targets

```rust
pub struct MyTarget;

impl TweenTarget for MyTarget {
    type Component = Transform;

    fn apply(value: f32, target: &mut Transform) {
        target.scale.x = value;
    }
}
```

Register:

```rust
app.add_systems(Update, tween_system::<MyTarget>);
```
