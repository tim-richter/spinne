import { Button } from 'my-library';

export const CustomButton = (props) => {
  return (
    <>
      <Button variant='blue' />
      <Button variant='blue' {...props} />
    </>
  );
};
