import React from "react"
import { IconsId } from "../types/icons"

export default function Icon({
  icon,
  className,
  ...rest
}: {
  icon: IconsId
  className?: string
}) {
  return (
    <i
      role="img"
      aria-label={icon}
      {...rest}
      className={`icon-${icon}` + (className ? ` ${className}` : "")}
    ></i>
  )
}