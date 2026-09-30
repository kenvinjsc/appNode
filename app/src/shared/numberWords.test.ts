import { describe, expect, it } from 'vitest';
import { moneyWords, numberToWords } from './numberWords';

describe('numberToWords', () => {
  it('đọc số nhỏ', () => {
    expect(numberToWords(0)).toBe('Không');
    expect(numberToWords(15)).toBe('Mười lăm');
    expect(numberToWords(21)).toBe('Hai mươi mốt');
    expect(numberToWords(105)).toBe('Một trăm lẻ năm');
    expect(numberToWords(1000)).toBe('Một nghìn');
  });
  it('đọc số tiền báo giá', () => {
    expect(moneyWords(29039040)).toBe('Hai mươi chín triệu không trăm ba mươi chín nghìn không trăm bốn mươi đồng');
    expect(numberToWords(1250000000)).toBe('Một tỷ hai trăm năm mươi triệu');
    expect(numberToWords(10035)).toBe('Mười nghìn không trăm ba mươi lăm');
  });
});
