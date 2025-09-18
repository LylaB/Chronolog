import NavbarComponent from "./NavbarComponent.tsx";
import HeroSection from "./sections/HeroSection.tsx";
import PillarsSection from "./sections/PillarsSection.tsx";

export default function HomePage() {
    return (
        <div>
            <NavbarComponent />
            <HeroSection />
            <PillarsSection />
        </div>
    );
}