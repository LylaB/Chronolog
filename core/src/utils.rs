#[macro_export]
macro_rules! policy_match {
    (
        $metadata: expr, $emitter: expr, $task: expr, 
        $self: expr, $result: expr, $policy_enum: ident
    ) => {{
        match ($result, &$self.policy) {
            (Err(error), &$policy_enum::RunUntilFailure) => {
                $emitter.clone().emit($metadata.clone(), $self.on_child_end.clone(), ($task.clone(), Some(error.clone()))).await;
                return Err(error)
            }
            (Ok(res), &$policy_enum::RunUntilSuccess) => {
                $emitter.clone().emit($metadata.clone(), $self.on_child_end.clone(), ($task.clone(), None)).await;
                return Ok(res)
            }
            (Err(error), _) => {
                $emitter.clone().emit($metadata.clone(), $self.on_child_end.clone(), ($task.clone(), Some(error.clone()))).await;
            }
            (Ok(res), _) => {
                $emitter.clone().emit($metadata.clone(), $self.on_child_end.clone(), ($task.clone(), None)).await;
            }
        }
    }};
}