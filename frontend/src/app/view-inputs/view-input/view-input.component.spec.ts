import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ViewInputComponent } from './view-input.component';

describe('ViewInputComponent', () => {
  let component: ViewInputComponent;
  let fixture: ComponentFixture<ViewInputComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ ViewInputComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(ViewInputComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
