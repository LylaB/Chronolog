import type {JSXElement} from "solid-js";

export default function IconButton(props: {
    color: "primary" | "neutral" | "warning";
    children: JSXElement;
    tooltip: string;
}) {
    let color: string = props.color;

    switch (props.color) {
        case "primary":
            color = "btn-primary border-primary/50";
            break;
        case "neutral":
            color = "border-neutral-content/50 hover:bg-neutral-content hover:text-neutral"
            break;
        case "warning":
            color = "btn-warning border-warning/50";
            break;
    }

    return (
        <div class={"tooltip tooltip-bottom"} data-tip={props.tooltip}>
            <button class={
                `btn btn-square btn-soft btn-sm text-xl md:btn-md md:text-[1.65rem] border-1 ${color} hover:text-2xl 
                md:hover:text-[1.85rem] duration-300 transition-all`
            }>
                {props.children}
            </button>
        </div>
    );
}