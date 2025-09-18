import type {JSXElement} from "solid-js";

export default function PillarStrengthCard(props: {
    title: string;
    description: string;
    icon: JSXElement,
    color: "primary" | "secondary" | "warning" | "success" | "info" | "accent" | "neutral" | "error"
}) {
    let iconBoxClass!: string;
    let hoverClass!: string;

    switch (props.color) {
        case "primary":
            iconBoxClass = "hover:bg-primary hover:text-primary-content bg-primary/10 border-primary text-primary"
            hoverClass = "peer-hover:text-primary"
            break;
        case "secondary":
            iconBoxClass = "hover:bg-secondary hover:text-secondary-content bg-secondary/10 border-secondary text-secondary"
            hoverClass = "peer-hover:text-secondary"
            break;
        case "accent":
            iconBoxClass = "hover:bg-accent hover:text-accent-content bg-accent/10 border-accent text-accent"
            hoverClass = "peer-hover:text-accent"
            break;
        case "warning":
            iconBoxClass = "hover:bg-warning hover:text-warning-content bg-warning/10 border-warning text-warning"
            hoverClass = "peer-hover:text-warning"
            break;
        case "info":
            iconBoxClass = "hover:bg-info hover:text-info-content bg-info/10 border-info text-info"
            hoverClass = "peer-hover:text-info"
            break;
        case "success":
            iconBoxClass = "hover:bg-success hover:text-success-content bg-success/10 border-success text-success"
            hoverClass = "peer-hover:text-success"
            break;
        case "error":
            iconBoxClass = "hover:bg-error hover:text-error-content bg-error/10 border-error text-error"
            hoverClass = "peer-hover:text-error"
            break;
        case "neutral":
            iconBoxClass = "hover:bg-neutral-content hover:text-neutral bg-neutral/90 border-neutral-content text-neutral-content"
            hoverClass = "peer-hover:opacity-40 peer-hover:text-base-content"
            break;
    }

    return (
        <div class={"flex flex-col items-center h-96"}>
            <div class={
                `size-32 rounded-lg border flex justify-center items-center p-5 ${iconBoxClass} ` +
                "hover:border-transparent hover:scale-110 cursor-pointer transition duration-300 mb-3 " +
                "peer group *:group-hover:scale-105 *:transition *:duration-300"
            }>
                {props.icon}
            </div>
            <div class={`text-center text-2xl xl:text-4xl font-extrabold font-[Rubik] ${hoverClass} duration-300 transition`}>
                {props.title}
            </div>
            <div class={
                "text-center text-sm w-74 lg:text-md lg:w-96 xl:text-lg xl:w-xl font-medium text-balance opacity-30 " +
                `font-[Rubik] duration-500 ${hoverClass} peer-hover:opacity-40 transition`
            }>
                {props.description}
            </div>
        </div>
    );
}