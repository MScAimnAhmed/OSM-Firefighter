import { ComponentFixture, TestBed } from '@angular/core/testing';

import { TurnInputComponent } from './turn-input.component';

describe('TurnInputComponent', () => {
  let component: TurnInputComponent;
  let fixture: ComponentFixture<TurnInputComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ TurnInputComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(TurnInputComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
