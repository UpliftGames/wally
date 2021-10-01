import React from "react"
import styled, { css } from "styled-components"
import { IconsId } from "../types/icons"
import Icon from "./Icon"

const ContainerStyle = css`
  position: relative;

  input:focus ~ i,
  select:focus ~ i {
    color: var(--wally-red);
  }
`

const InputStyle = css`
  width: 100%;
  padding: 0.25rem 1rem;
  border: 2px solid var(--wally-mauve);

  &:focus {
    outline: none;
    border-color: var(--wally-red);
  }
`

const InputContainer = styled.div`
  ${ContainerStyle}

  i {
    position: absolute;
    right: 1rem;
    font-size: 1.4rem;
    line-height: 2.6rem;
  }
`

const Input = styled.input.attrs({
  type: "text",
})`
  ${InputStyle}

  border-radius: 50px;
`

export function TextInput({
  icon,
  placeholder,
  value,
  onChange,
}: {
  icon: IconsId
  value: string
  onChange: (value: string) => unknown
  placeholder?: string
}) {
  return (
    <InputContainer>
      <Input
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <Icon icon={icon} />
    </InputContainer>
  )
}

const SelectContainer = styled.div`
  ${ContainerStyle}

  i {
    position: absolute;
    right: 0.6rem;
    font-size: 0.7rem;
    line-height: 2.4rem;
    pointer-events: none;
  }
`

const Select = styled.select`
  ${InputStyle}

  appearance: none;
  padding-right: 2.5rem;
  cursor: pointer;

  &:focus {
    color: var(--wally-red);
  }
`

export function SelectInput<T extends string>({
  options,
  placeholder,
  onChange,
}: {
  placeholder: string
  options: {
    label: string
    value: T
  }[]
  onChange: (value: T | undefined) => unknown
}) {
  return (
    <SelectContainer>
      <Select
        onChange={(e) =>
          onChange(e.target.value === "" ? undefined : (e.target.value as T))
        }
      >
        {[
          {
            label: placeholder,
            value: "",
          },
          ...options,
        ].map((option) => (
          <option value={option.value} key={option.value}>
            {option.label}
          </option>
        ))}
      </Select>
      <Icon icon="chevron" />
    </SelectContainer>
  )
}
